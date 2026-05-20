//! Configuration management module.
//!
//! This module implements a **layered configuration architecture** for the Minecraft server management.
//!
//! **Load Priority (from lowest to highest):**
//! 1. **Internal Defaults** (Hardcoded values in Rust `impl Default`).
//! 2. **OS Configuration Directory** (e.g., `~/.config/mc_server/config.toml`).
//! 3. **Environment-specific file** (e.g., `config/dev.toml` or `config/prod.toml`).
//! 4. **Local override file** `config/local.toml` (git-ignored, for personal overrides).
//! 5. **Environment Variables** (Prefix `APP__`, e.g., `APP__SERVER__PORT`).

use crate::error::{Error, Result};
use config::{Config, Environment, File};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod error;

/// Main application configuration.
///
/// Groups all configuration sections required to run and manage
/// the Minecraft server and its auxiliary tools.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McConfig {
    /// Configuration regarding the Java Virtual Machine (JVM).
    #[serde(default)]
    pub java: JavaCfg,

    /// Specific configuration for the game server (directories, jar, eula).
    #[serde(default)]
    pub server: ServerCfg,

    /// Configuration for the command line interface or remote connection.
    #[serde(default)] // Added serde default here for consistency
    pub client: ClientCfg,
}

impl Default for McConfig {
    fn default() -> Self {
        Self {
            java: JavaCfg::default(),
            server: ServerCfg::default(),
            client: ClientCfg::default(),
        }
    }
}

/// Java execution configuration options.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JavaCfg {
    /// Path to the java executable (e.g., "/usr/bin/java" or simply "java").
    pub path: String,
    /// Initial memory allocation (flag -Xms).
    pub xms: String,
    /// Maximum memory allocation (flag -Xmx).
    pub xmx: String,
    /// Additional arguments for the JVM (e.g., Garbage Collector flags).
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl Default for JavaCfg {
    fn default() -> Self {
        Self {
            path: "java".to_string(),
            xms: "1G".to_string(),
            xmx: "2G".to_string(),
            extra_args: vec![],
        }
    }
}

/// Minecraft server operational options.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerCfg {
    /// Minecraft version.
    /// Defaults to `1.21.10`
    pub version: String,
    /// Root directory where server files are hosted.
    /// Defaults to `runtime/server`.
    pub working_dir: PathBuf,
    /// Name of the server .jar file (e.g., "server.jar").
    pub jar: PathBuf,
    /// If `true`, adds the `-nogui` flag at startup.
    #[serde(default)]
    pub nogui: bool,
    /// If `true`, automatically accepts Mojang's EULA by writing to the eula.txt file.
    #[serde(default)]
    pub auto_eula: bool,
}

impl Default for ServerCfg {
    fn default() -> Self {
        Self {
            version: "1.21.10".to_string(),
            working_dir: PathBuf::from("runtime/server"),
            jar: PathBuf::from("server.jar"),
            nogui: true,
            auto_eula: false,
        }
    }
}

/// Network configuration for the internal CLI/API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientCfg {
    /// Host address to bind to (e.g., "127.0.0.1" or "0.0.0.0").
    pub host: String,
    /// Listening port. Defaults to 7110.
    pub port: u16,
}

impl Default for ClientCfg {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 7110,
        }
    }
}

impl ClientCfg {
    /// Returns the complete socket address as a string (e.g., "127.0.0.1:7110").
    pub fn get_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl McConfig {
    /// Initializes and loads the configuration by merging multiple sources.
    ///
    /// # Load Strategy
    /// 1. **Internal Defaults**: Loads hardcoded values from `impl Default`.
    /// 2. **Dotenv**: Loads variables from `.env` if it exists.
    /// 3. **System**: Searches for configuration in the standard user directory.
    ///    - Linux: `~/.config/mc_server/config.{toml|...}`
    ///    - Windows: `%APPDATA%\pllinas\mc_server\config.{toml|...}`
    ///    - macOS: `~/Library/Application Support/pllinas.mc_server/config.{toml|...}`
    /// 4. **Run Mode**: Based on the `RUN_MODE` environment variable (default: "dev").
    ///    - Loads `config/{RUN_MODE}.{toml|...}` (e.g., `config/dev.toml`).
    /// 5. **Local**: Loads `config/local.{toml|...}` (usually git-ignored).
    /// 6. **Environment Variables**: Overrides values using the `APP` prefix.
    ///    - E.g., `APP__SERVER__PORT` overrides `server.port`.
    ///
    /// # Errors
    /// Returns a `crate::error::Error` if there are syntax issues in the files
    /// or incorrect data types.
    pub fn new() -> Result<Self> {
        dotenvy::dotenv().ok();
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "dev".to_string());

        // Define standard OS paths
        // Linux: ~/.config/mc_server
        // Windows: Roaming/pllinas/mc_server
        let project_dir = ProjectDirs::from("", "pllinas", "mc_server");

        let mut builder = Config::builder();

        // Layer 1: Internal Defaults (Hardcoded)
        let default_config = Config::try_from(&McConfig::default()).map_err(Error::from)?;

        builder = builder.add_source(default_config);

        // Layer 2: OS System Configuration
        if let Some(proj_dirs) = project_dir {
            let config_path = proj_dirs.config_dir().join("config");
            builder = builder.add_source(File::from(config_path).required(false));
        };

        builder = builder
            // Layer 3: Run Mode (dev, prod, etc.)
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // Layer 4: Local Overrides
            .add_source(File::with_name("config/local").required(false))
            // Layer 5: Environment Variables
            .add_source(Environment::with_prefix("APP").separator("__"));

        let s = builder.build()?;
        s.try_deserialize().map_err(|e| Error::from(e))
    }
}
