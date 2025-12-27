use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Error trying to build the config object: {0}")]
    ErrorBuildingConfigFile(#[from] config::ConfigError),
}

pub type Result<T> = std::result::Result<T, Error>;
