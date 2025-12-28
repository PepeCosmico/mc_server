use crate::error::{Error, Result};
use serde::Deserialize;
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub java: JavaCfg,
    pub server: ServerCfg,
    pub backup: BackupCfg,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JavaCfg {
    pub path: String,
    pub xms: String,
    pub xmx: String,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerCfg {
    pub working_dir: String,
    pub jar: String,
    pub nogui: bool,
    pub auto_eula: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackupCfg {
    pub path: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let s = std::fs::read_to_string(path).map_err(Error::ReadConfigFailed)?;
        Ok(toml::from_str(&s).map_err(Error::DeserConfigFailed)?)
    }
}
