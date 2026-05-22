//! Orphan JVM detection and recovery at daemon startup.
//!
//! If the daemon dies abruptly (`kill -9`, OOM, host reboot) the JVM child is
//! orphaned: the parent's `kill_on_drop` never fires and port 25565 stays
//! bound. On the next daemon start we read the pidfile and decide what to do
//! with whatever process (if any) the recorded PID points to.

use mc_config::OrphanPolicy;
use mc_process::pidfile;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
#[cfg(unix)]
use sysinfo::Signal;
use tracing::{debug, info, warn};

/// Run the orphan-recovery flow before the actor is spawned.
///
/// Returns `Ok(())` if the daemon is safe to continue starting. Returns
/// `Err(_)` only when the configured policy is `Refuse` and a real orphan was
/// found — the caller is expected to translate that into a process exit.
pub fn run(working_dir_raw: &Path, policy: OrphanPolicy) -> anyhow::Result<()> {
    debug!("working_dir = {}", working_dir_raw.display());
    debug!("policy = {policy:?}");

    let pid = match pidfile::read(working_dir_raw) {
        Ok(Some(pid)) => {
            debug!("pidfile pid = {pid}");
            pid
        }
        Ok(None) => {
            debug!("no pidfile, clean start");
            return Ok(());
        }
        Err(e) => {
            warn!("pidfile unreadable ({e}), assuming clean start");
            return Ok(());
        }
    };

    let working_dir = match dunce::canonicalize(working_dir_raw) {
        Ok(p) => {
            debug!("canonical working_dir = {}", p.display());
            p
        }
        Err(e) => {
            warn!("working_dir does not exist ({e}), removing stale pidfile");
            let _ = pidfile::remove(working_dir_raw);
            return Ok(());
        }
    };

    let mut sys = System::new();
    match classify(pid, &working_dir, &mut sys) {
        Orphan::None => {
            debug!("stale pidfile, removing");
            let _ = pidfile::remove(working_dir_raw);
            Ok(())
        }
        Orphan::Alive => match policy {
            OrphanPolicy::Refuse => Err(anyhow::anyhow!(
                "orphan jvm pid {pid} in {}\n\
                 kill it or delete {}, or set [daemon] orphan_policy = \"kill\"",
                working_dir.display(),
                pidfile_display(working_dir_raw),
            )),
            OrphanPolicy::Kill => {
                info!("killing orphan jvm pid {pid}");
                kill(pid, &mut sys)?;
                let _ = pidfile::remove(working_dir_raw);
                info!("orphan killed");
                Ok(())
            }
        },
    }
}

enum Orphan {
    /// PID dead, or the live process is not a JVM rooted at our working_dir.
    /// Treat as a stale pidfile — clean up and proceed.
    None,
    /// Live `java` whose cwd matches our `working_dir`. The real deal.
    Alive,
}

fn classify(pid: u32, working_dir: &Path, sys: &mut System) -> Orphan {
    let sys_pid = Pid::from(pid as usize);

    // Pedimos cwd explícitamente — el refresh por defecto no lo puebla
    // siempre en Windows.
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[sys_pid]),
        true,
        ProcessRefreshKind::nothing()
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always),
    );

    let Some(proc) = sys.process(sys_pid) else {
        debug!("classify: pid {pid} not found");
        return Orphan::None;
    };

    let name = proc.name().to_string_lossy().to_ascii_lowercase();
    debug!("classify: pid {pid} name = {name:?}");
    if !name.contains("java") {
        debug!("classify: not a java process");
        return Orphan::None;
    }

    let proc_cwd = proc.cwd().map(|p| p.to_path_buf());
    debug!("classify: pid {pid} cwd = {proc_cwd:?}");

    let canonical_cwd = proc_cwd
        .as_deref()
        .and_then(|cwd| dunce::canonicalize(cwd).ok());

    match canonical_cwd {
        Some(cwd) if cwd == working_dir => {
            debug!("classify: cwd matches, orphan confirmed");
            Orphan::Alive
        }
        Some(cwd) => {
            debug!(
                "classify: cwd mismatch (expected {}, got {})",
                working_dir.display(),
                cwd.display()
            );
            Orphan::None
        }
        None => {
            // En Windows leer cwd de un proceso re-parentado a system suele
            // fallar. El pidfile vive dentro de nuestro working_dir, así que
            // pidfile + PID vivo + nombre java ya es señal fuerte. Confiamos.
            debug!("classify: cwd unavailable, trusting pidfile");
            Orphan::Alive
        }
    }
}

fn kill(pid: u32, sys: &mut System) -> anyhow::Result<()> {
    let sys_pid = Pid::from(pid as usize);

    #[cfg(unix)]
    {
        sys.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);
        let Some(proc) = sys.process(sys_pid) else {
            debug!("kill: pid {pid} already gone");
            return Ok(());
        };
        debug!("kill: sending SIGTERM to pid {pid}");
        if !proc.kill_with(Signal::Term).unwrap_or(false) {
            return Err(anyhow::anyhow!("SIGTERM failed for pid {pid}"));
        }
    }
    #[cfg(not(unix))]
    debug!("kill: skipping graceful signal (not supported on this platform)");

    debug!("kill: waiting up to 30s for graceful exit");
    let graceful_start = Instant::now();
    let graceful_deadline = graceful_start + Duration::from_secs(30);
    let mut last_tick = graceful_start;
    while Instant::now() < graceful_deadline {
        std::thread::sleep(Duration::from_millis(200));
        sys.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);
        if sys.process(sys_pid).is_none() {
            debug!(
                "kill: pid {pid} exited after {:.1}s",
                graceful_start.elapsed().as_secs_f32()
            );
            return Ok(());
        }
        if last_tick.elapsed() >= Duration::from_secs(5) {
            debug!(
                "kill: pid {pid} still alive after {:.0}s",
                graceful_start.elapsed().as_secs_f32()
            );
            last_tick = Instant::now();
        }
    }

    debug!("kill: graceful timeout, escalating to force kill");
    sys.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);
    if let Some(proc) = sys.process(sys_pid) {
        if !proc.kill() {
            return Err(anyhow::anyhow!("force kill failed for pid {pid}"));
        }
    } else {
        return Ok(());
    }

    let force_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < force_deadline {
        std::thread::sleep(Duration::from_millis(100));
        sys.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);
        if sys.process(sys_pid).is_none() {
            debug!("kill: pid {pid} exited after force kill");
            return Ok(());
        }
    }
    Err(anyhow::anyhow!("pid {pid} ignored force kill"))
}

fn pidfile_display(working_dir_raw: &Path) -> String {
    let mut p = PathBuf::from(working_dir_raw);
    p.push("runtime");
    p.push("server.pid");
    p.display().to_string()
}
