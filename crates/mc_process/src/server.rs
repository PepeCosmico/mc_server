use crate::error::{Error, Result};
use crate::logs::McLog;
use crate::state::{McVersion, ServerState};
use crate::{metrics, process};
use mc_config::McConfig;
use std::sync::Arc;
use sysinfo::System;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::{Notify, broadcast, watch};

/// Owner of the Minecraft JVM child process.
///
/// `ServerProcess` is the single mutator of the child handle. It exposes
/// imperative commands (`start`, `stop`, `exec_command`, `op`, `get_metrics`)
/// and three subscription channels (`state`, `version`, `logs`) for
/// observers — typically the daemon actor.
#[derive(Debug)]
pub struct ServerProcess {
    cfg: McConfig,
    system: System,
    stdin: Option<ChildStdin>,
    state_tx: watch::Sender<ServerState>,
    version_tx: watch::Sender<Option<McVersion>>,
    log_tx: broadcast::Sender<McLog>,
    process_id: Option<u32>,
    kill_signal: Arc<Notify>,
}

impl ServerProcess {
    pub fn new(cfg: McConfig) -> Self {
        let (state_tx, _) = watch::channel(ServerState::Stopped);
        let (version_tx, _) = watch::channel(None);
        let (log_tx, _) = broadcast::channel(256);
        Self {
            cfg,
            system: System::new(),
            stdin: None,
            state_tx,
            version_tx,
            log_tx,
            process_id: None,
            kill_signal: Arc::new(Notify::new()),
        }
    }

    /// Subscribe to state changes.
    pub fn state(&self) -> watch::Receiver<ServerState> {
        self.state_tx.subscribe()
    }

    /// Subscribe to version changes.
    pub fn version(&self) -> watch::Receiver<Option<McVersion>> {
        self.version_tx.subscribe()
    }

    /// Subscribe to typed log entries (stdout + stderr).
    pub fn logs(&self) -> broadcast::Receiver<McLog> {
        self.log_tx.subscribe()
    }

    /// Spawn the Java process and the supporting logger/reaper tasks.
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        process::prepare_workdir(&self.cfg).await?;

        let (working_dir, jar_path) =
            process::resolve_paths(&self.cfg).await.inspect_err(|e| {
                if matches!(e, Error::JarFileDoesNotExist) {
                    self.state_tx.send_replace(ServerState::Stopped);
                }
            })?;

        let args = process::build_jvm_args(&self.cfg, &jar_path);
        let mut child = process::spawn_jvm(&self.cfg, &working_dir, &args)?;

        self.process_id = child.id();
        self.stdin = child.stdin.take();

        let stdout = child.stdout.take().expect("child stdout missing");
        let stderr = child.stderr.take().expect("child stderr missing");

        process::spawn_logger(
            stdout,
            stderr,
            self.log_tx.clone(),
            self.state_tx.clone(),
            self.version_tx.clone(),
        );
        process::spawn_reaper(
            child,
            self.state_tx.clone(),
            self.version_tx.clone(),
            self.kill_signal.clone(),
        );

        Ok(())
    }

    /// Send `/stop` to the server.
    pub async fn stop(&mut self) -> Result<()> {
        if matches!(*self.state_tx.borrow(), ServerState::Stopped) {
            return Ok(());
        }
        self.exec_command("stop").await
    }

    /// Send `/op <player>` or `/deop <player>`.
    pub async fn op(&mut self, op: bool, player_name: String) -> Result<()> {
        if player_name.contains(char::is_whitespace) {
            return Err(Error::WriteStdinFailed(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Player name cannot contain whitespace",
            )));
        }
        let action = if op { "op" } else { "deop" };
        self.exec_command(&format!("{} {}", action, player_name))
            .await
    }

    /// Send an arbitrary command to the JVM's stdin (e.g. "list", "say hello").
    pub async fn exec_command(&mut self, cmd: &str) -> Result<()> {
        if !self.is_running() {
            return Err(Error::WriteWhileNotRunningError);
        }
        let clean_cmd = cmd.replace('\n', "");
        if let Some(stdin) = &mut self.stdin {
            stdin
                .write_all(clean_cmd.as_bytes())
                .await
                .map_err(Error::WriteStdinFailed)?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(Error::WriteStdinFailed)?;
            stdin.flush().await.map_err(Error::WriteStdinFailed)?;
        }
        Ok(())
    }

    /// CPU usage (%) and memory (bytes) of the JVM, when running.
    pub fn get_metrics(&mut self) -> Option<(f32, u64)> {
        if !self.is_running() {
            return None;
        }
        let pid = self.process_id?;
        metrics::read(&mut self.system, pid)
    }

    fn is_running(&self) -> bool {
        matches!(
            *self.state_tx.borrow(),
            ServerState::Running | ServerState::Starting
        )
    }
}
