use mc_config::McConfig;
use mc_process::server::ServerProcess;
use std::sync::OnceLock;

pub struct Daemon {
    cfg: OnceLock<McConfig>,
    server: OnceLock<ServerProcess>,
}

impl Daemon {
    pub const fn new() -> Self {
        Self {
            cfg: OnceLock::new(),
            server: OnceLock::new(),
        }
    }

    pub fn init(&self, cfg: McConfig) {
        self.cfg
            .set(cfg.clone())
            .expect("Error initializing config");
        self.server
            .set(ServerProcess::new(cfg))
            .expect("Error initializing server");
    }

    pub fn get_cfg(&self) -> McConfig {
        self.cfg.get().expect("")
    }
}
