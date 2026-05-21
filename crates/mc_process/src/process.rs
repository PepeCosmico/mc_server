use crate::error::{Error, Result};
use crate::logs::{McLog, McLogParser};
use chrono::Local;
use mc_config::McConfig;
use mc_types::server::{event::ServerEvent, state::ServerState, version::McVersion};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::select;
use tokio::sync::{broadcast, watch, Notify};

/// Create the server's working directory and (optionally) write `eula.txt`.
pub(crate) async fn prepare_workdir(cfg: &McConfig) -> Result<()> {
    let dir = PathBuf::from(&cfg.server.working_dir);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(Error::CreateWorkingDirectoryFailed)?;

    if cfg.server.auto_eula {
        tokio::fs::write(dir.join("eula.txt"), "eula=true\n")
            .await
            .map_err(Error::CreateEulaFileFailed)?;
    }
    Ok(())
}

/// Canonicalize the working directory and verify the jar exists.
pub(crate) async fn resolve_paths(cfg: &McConfig) -> Result<(PathBuf, PathBuf)> {
    let raw_path = PathBuf::from(&cfg.server.working_dir);
    let working_dir =
        dunce::canonicalize(&raw_path).map_err(Error::ResolveWorkingDirectoryFailed)?;
    let jar_path = working_dir.join(&cfg.server.jar);
    if !jar_path.exists() {
        return Err(Error::JarFileDoesNotExist);
    }
    Ok((working_dir, jar_path))
}

/// Build the argv passed to the JVM.
pub(crate) fn build_jvm_args(cfg: &McConfig, jar_path: &Path) -> Vec<String> {
    let mut args = vec![
        format!("-Xms{}", cfg.java.xms),
        format!("-Xmx{}", cfg.java.xmx),
    ];
    args.extend(cfg.java.extra_args.clone());
    args.push("-jar".to_string());
    args.push(jar_path.to_string_lossy().to_string());
    if cfg.server.nogui {
        args.push("nogui".to_string());
    }
    args
}

/// Spawn the JVM with piped stdio and `kill_on_drop`.
pub(crate) fn spawn_jvm(
    cfg: &McConfig,
    working_dir: &Path,
    args: &[String],
) -> Result<tokio::process::Child> {
    Command::new(&cfg.java.path)
        .args(args)
        .current_dir(working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(Error::SpawnFailed)
}

/// Spawn the task that awaits child exit and reflects it in `state_tx`.
/// Also reacts to `kill_signal` by force-killing the child.
pub(crate) fn spawn_reaper(
    mut child: tokio::process::Child,
    state_tx: watch::Sender<ServerState>,
    version_tx: watch::Sender<Option<McVersion>>,
    kill_signal: Arc<Notify>,
) {
    tokio::spawn(async move {
        loop {
            select! {
                exit_status = child.wait() => {
                    version_tx.send_replace(None);
                    let next = match exit_status {
                        Ok(s) if s.success() => ServerState::Stopped,
                        _ => ServerState::Crashed,
                    };
                    state_tx.send_replace(next);
                    break;
                }
                _ = kill_signal.notified() => {
                    let _ = child.start_kill();
                }
            }
        }
    });
}

/// Spawn the task that consumes stdout+stderr, parses log lines, drives state
/// transitions, and forwards typed [`McLog`] entries on `log_tx`.
pub(crate) fn spawn_logger(
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    log_tx: broadcast::Sender<McLog>,
    state_tx: watch::Sender<ServerState>,
    version_tx: watch::Sender<Option<McVersion>>,
) {
    tokio::spawn(async move {
        let mut out_reader = BufReader::new(stdout).lines();
        let mut err_reader = BufReader::new(stderr).lines();
        loop {
            select! {
                Ok(Some(line)) = out_reader.next_line() => {
                    handle_stdout_line(line, &log_tx, &state_tx, &version_tx);
                }
                Ok(Some(line)) = err_reader.next_line() => {
                    handle_stderr_line(line, &log_tx);
                }
                else => break,
            }
        }
    });
}

fn handle_stdout_line(
    line: String,
    log_tx: &broadcast::Sender<McLog>,
    state_tx: &watch::Sender<ServerState>,
    version_tx: &watch::Sender<Option<McVersion>>,
) {
    let log = match McLogParser::parse(&line) {
        Some(parsed) => {
            apply_event(&parsed.event, state_tx, version_tx);
            parsed
        }
        None => McLog {
            timestamp: String::new(),
            level: "RAW".to_string(),
            message: line,
            event: ServerEvent::Unknown,
        },
    };
    let _ = log_tx.send(log);
}

fn handle_stderr_line(line: String, log_tx: &broadcast::Sender<McLog>) {
    let log = McLog {
        timestamp: Local::now().to_rfc3339(),
        level: "STDERR".to_string(),
        message: line,
        event: ServerEvent::Unknown,
    };
    let _ = log_tx.send(log);
}

/// Apply the pure state transition and any associated metadata side-effects
/// (version_tx is populated on `Stopped → Starting`).
fn apply_event(
    event: &ServerEvent,
    state_tx: &watch::Sender<ServerState>,
    version_tx: &watch::Sender<Option<McVersion>>,
) {
    let current = *state_tx.borrow();
    if let Some(next) = current.next(event) {
        state_tx.send_replace(next);
        if let ServerEvent::Starting {
            mc_version,
            fabric_version,
        } = event
        {
            version_tx.send_replace(Some(McVersion {
                mc_version: mc_version.clone(),
                fabric_version: fabric_version.clone(),
            }));
        }
    }
}
