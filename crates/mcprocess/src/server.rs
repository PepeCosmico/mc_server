use crate::config::Config;
use crate::error::{Error, Result};
use crate::logs::{McLog, McLogParser, ServerEvent};
pub use crate::ServerState;
use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, Command},
    select,
    sync::{broadcast, watch, Notify},
    time::{timeout, Duration},
};

pub struct ServerProcess {
    cfg: Config,
    system: System,
    stdin: Option<ChildStdin>,
    state_tx: watch::Sender<ServerState>,
    log_tx: broadcast::Sender<String>,
    process_id: Option<u32>,
    saved_signal: Arc<Notify>,
    kill_signal: Arc<Notify>,
}

impl ServerProcess {
    pub fn new(cfg: Config) -> Self {
        let (state_tx, _state_rx) = watch::channel(ServerState::Stopped);
        let (log_tx, _log_rx) = broadcast::channel(256);
        Self {
            cfg,
            system: System::new(),
            stdin: None,
            state_tx,
            log_tx,
            process_id: None,
            saved_signal: Arc::new(Notify::new()),
            kill_signal: Arc::new(Notify::new()),
        }
    }

    /// Subscribe to state changes.
    pub fn state(&self) -> watch::Receiver<ServerState> {
        self.state_tx.subscribe()
    }

    /// Subscribe to log lines (stdout & stderr).
    pub fn logs(&self) -> broadcast::Receiver<String> {
        self.log_tx.subscribe()
    }

    /// Prepare filesystem basics (dir, eula…).
    async fn prepare(&self) -> Result<()> {
        let dir = PathBuf::from(&self.cfg.server.working_dir);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|err| Error::CreateWorkingDirectoryFailed(err))?;

        if self.cfg.server.auto_eula {
            tokio::fs::write(dir.join("eula.txt"), "eula=true\n")
                .await
                .map_err(Error::CreateEulaFileFailed)?;
        }

        Ok(())
    }

    /// Start the Java/Fabric process.
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        self.state_tx.send_replace(ServerState::Starting);
        self.prepare().await?;

        let (working_dir, jar_path) = self.resolve_paths().await?;
        let args = self.build_jvm_args(&jar_path);

        println!("Ejecutando Java en: {:?}", working_dir);

        let mut child = self.spawn_child(&working_dir, &args)?;

        self.process_id = child.id();
        self.stdin = child.stdin.take();

        let stdout = child.stdout.take().expect("child stdout missing");
        let stderr = child.stderr.take().expect("child stderr missing");

        self.spawn_logger_task(stdout, stderr);
        self.spawn_reaper_task(child);

        Ok(())
    }

    /// Send `/stop` to the server and wait a bit.
    pub async fn stop(&mut self, timeout_secs: u64) -> Result<()> {
        if matches!(*self.state_tx.borrow(), ServerState::Stopped) {
            return Ok(());
        }
        self.exec_command("stop").await?;

        let mut state_rx = self.state_tx.subscribe();

        let duration = Duration::from_secs(timeout_secs);

        let wait_result = timeout(duration, async {
            while let Ok(()) = state_rx.changed().await {
                let state = *state_rx.borrow();
                if matches!(state, ServerState::Stopped | ServerState::Crashed) {
                    return;
                }
            }
        })
            .await;

        match wait_result {
            Ok(_) => {
                println!("Servidor detenido correctamente.");
                Ok(())
            }
            Err(_) => {
                eprintln!(
                    "El servidor no respondió al stop en {}s. Forzando cierre (KILL)...",
                    timeout_secs
                );
                self.kill_signal.notify_one();
                Ok(())
            }
        }
    }

    /// Send arbitrary command to stdin (e.g. "list", "say hello", …).
    pub async fn exec_command(&mut self, cmd: &str) -> Result<()> {
        if let Some(stdin) = &mut self.stdin {
            stdin
                .write_all(cmd.as_bytes())
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

    /// Realiza un backup completo del servidor de forma segura.
    pub async fn backup(&mut self) -> Result<String> {
        let backup_dir = PathBuf::from(&self.cfg.backup.path);

        if !backup_dir.exists() {
            tokio::fs::create_dir_all(&backup_dir)
                .await
                .map_err(Error::CreateBackupDirFailed)?;
        }

        let now = Local::now();
        let filename = format!("backup_{}.tar.gz", now.format("%Y-%m-%d_%H-%M-%S"));
        let backup_path = backup_dir.join(&filename);

        // 3. Preparar el servidor (Hot Backup)
        let is_running = self.is_running();
        if is_running {
            println!("Servidor activo: desactivando auto-save...");
            self.exec_command("save-off").await?;

            // IMPORTANTE: Preparamos la espera ANTES de enviar el comando
            // para no perdernos la notificación si es instantánea.
            let save_notify = self.saved_signal.clone();
            let save_waiter = save_notify.notified();

            self.exec_command("save-all").await?;

            println!("Esperando confirmación de guardado...");

            // Esperamos a que suene el timbre (con un timeout de seguridad)
            match timeout(Duration::from_secs(10), save_waiter).await {
                Ok(_) => println!("✅ Guardado confirmado."),
                Err(_) => eprintln!("⚠️ Timeout esperando guardado. Continuando..."),
            }
        }

        println!("Iniciando compresión en: {:?}", backup_path);

        let working_dir = dunce::canonicalize(&self.cfg.server.working_dir)
            .map_err(Error::ResolveWorkingDirectoryFailed)?;

        let backup_path_clone = backup_path.clone();

        tokio::task::spawn_blocking(move || {
            ServerProcess::create_archive(&working_dir, &backup_path_clone)
        })
            .await
            .map_err(Error::BackupTaskFailed)??;

        if is_running {
            println!("Backup finalizado. Reactivando auto-save...");
            self.exec_command("save-on").await?;
        }

        println!("Backup completado exitosamente: {}", filename);
        Ok(filename)
    }

    pub fn get_metrics(&mut self) -> Option<(f32, u64)> {
        if let Some(pid_val) = self.process_id {
            let pid = Pid::from(pid_val as usize);

            self.system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

            if let Some(process) = self.system.process(pid) {
                return Some((process.cpu_usage(), process.memory()));
            }
        }
        None
    }

    // PRIVATE METHODS

    /// Verifica si el servidor está en un estado activo
    fn is_running(&self) -> bool {
        matches!(
            *self.state_tx.borrow(),
            ServerState::Running | ServerState::Starting | ServerState::Loading
        )
    }

    /// Función helper (síncrona) para crear el tar.gz
    /// Esta función se ejecuta en un hilo separado.
    fn create_archive(source_dir: &Path, dest_path: &Path) -> Result<()> {
        let tar_gz = File::create(dest_path).map_err(Error::CreateArchiveFailed)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);

        tar.append_dir_all("server", source_dir)
            .map_err(Error::CreateArchiveFailed)?;

        Ok(())
    }

    /// Resuelve rutas absolutas y verifica existencia del JAR
    async fn resolve_paths(&self) -> Result<(PathBuf, PathBuf)> {
        let raw_path = PathBuf::from(&self.cfg.server.working_dir);
        let working_dir =
            dunce::canonicalize(&raw_path).map_err(Error::ResolveWorkingDirectoryFailed)?;

        let jar_path = working_dir.join(&self.cfg.server.jar);

        if !jar_path.exists() {
            let _ = self.state_tx.send(ServerState::Stopped);
            return Err(Error::JarFileDoesNotExist);
        }

        Ok((working_dir, jar_path))
    }

    /// Construye el vector de argumentos para Java
    fn build_jvm_args(&self, jar_path: &Path) -> Vec<String> {
        let mut args = vec![
            format!("-Xms{}", self.cfg.java.xms),
            format!("-Xmx{}", self.cfg.java.xmx),
        ];
        args.extend(self.cfg.java.extra_args.clone());
        args.push("-jar".to_string());
        args.push(jar_path.to_string_lossy().to_string());

        if self.cfg.server.nogui {
            args.push("nogui".to_string());
        }
        args
    }

    /// Configura y lanza el Command
    fn spawn_child(&self, working_dir: &Path, args: &[String]) -> Result<tokio::process::Child> {
        Command::new(&self.cfg.java.path)
            .args(args)
            .current_dir(working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true) // ¡Seguridad ante crashes!
            .spawn()
            .map_err(|err| Error::SpawnFailed(err))
    }

    /// Inicia la tarea que lee logs y detecta el estado "Done"
    fn spawn_logger_task(
        &self,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
    ) {
        let log_tx = self.log_tx.clone();
        let state_tx = self.state_tx.clone();
        let save_notify = self.saved_signal.clone(); // Clonamos para mover al closure

        tokio::spawn(async move {
            let mut out_reader = BufReader::new(stdout).lines();
            let mut err_reader = BufReader::new(stderr).lines();

            // Marcamos Loading al inicio
            state_tx.send_replace(ServerState::Loading);

            loop {
                select! {
                    // --- STDOUT ---
                    line = out_reader.next_line() => {
                        match line {
                            Ok(Some(raw_line)) => {
                                // 1. Parseamos la línea con nuestro nuevo módulo
                                let log_obj = if let Some(parsed) = McLogParser::parse(&raw_line) {

                                    // 2. REACCIONAMOS A EVENTOS (State Machine)
                                    match parsed.event {
                                        ServerEvent::Ready(_) => {
                                            // Solo pasamos a running si estábamos cargando
                                            if *state_tx.borrow() == ServerState::Loading {
                                                let _ = state_tx.send(ServerState::Running);
                                            }
                                        },
                                        ServerEvent::Stopping => {
                                            let _ = state_tx.send(ServerState::Stopping);
                                        },
                                        ServerEvent::Saving => {
                                            let current = *state_tx.borrow();
                                            if current == ServerState::Running {
                                                let _ = state_tx.send(ServerState::Saving);
                                            }
                                        }
                                        ServerEvent::Saved => {
                                            let current = *state_tx.borrow();
                                            if current == ServerState::Saving {
                                                let _ = state_tx.send(ServerState::Running);
                                                save_notify.notify_waiters();
                                            }
                                        }
                                        // Aquí podrías añadir lógica extra, ej:
                                        // ServerEvent::Joined(player) => println!("¡Entró {}!", player),
                                        _ => {}
                                    }
                                    parsed // Usamos el objeto parseado
                                } else {
                                    // Si falla el regex (línea rara), creamos un genérico
                                    McLog {
                                        timestamp: "".to_string(),
                                        level: "RAW".to_string(),
                                        message: raw_line,
                                        event: ServerEvent::Unknown
                                    }
                                };

                                // 3. Enviamos el JSON limpio a los clientes (Daemon/Frontend)
                                let _ = log_tx.send(log_obj.to_json());
                            }
                            _ => break, // EOF
                        }
                    }
                    // --- STDERR ---
                    line = err_reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                // Empaquetamos stderr como un evento JSON también
                                let err_entry = McLog {
                                    timestamp: "".to_string(),
                                    level: "STDERR".to_string(),
                                    message: l,
                                    event: ServerEvent::Unknown,
                                };
                                let _ = log_tx.send(err_entry.to_json());
                            }
                            _ => break,
                        }
                    }
                }
            }
        });
    }

    /// Inicia la tarea que espera a que el proceso muera
    fn spawn_reaper_task(&self, mut child: tokio::process::Child) {
        let state_tx = self.state_tx.clone();
        let kill_signal = self.kill_signal.clone();

        tokio::spawn(async move {
            loop {
                select! {
                    exit_status = child.wait() => {
                        match exit_status {
                            Ok(s) if s.success() => { let _ = state_tx.send(ServerState::Stopped); },
                            Ok(_) => { let _ = state_tx.send(ServerState::Crashed); },
                            Err(_) => { let _ = state_tx.send(ServerState::Crashed); }
                        }
                        break;
                    }
                    _ = kill_signal.notified() => {
                        let _ = child.start_kill();
                    }
                }
            }
        });
    }
}
