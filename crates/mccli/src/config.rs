use config::{Config, Environment, File};
use serde::Deserialize;
use tracing::{debug, info};

use crate::error::Result;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        debug!("Loading configuration...");

        let builder = Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .add_source(File::with_name("config.toml").required(false))
            .add_source(Environment::with_prefix("MCCLI").separator("__"));

        let config = builder.build()?;

        let app_config: AppConfig = config.try_deserialize()?;
        info!("Loaded configuration");

        Ok(app_config)
    }

    pub fn get_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
