use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Error connecting to daemon {0}")]
    ErrorConnectingToDaemon(#[from] std::io::Error),
    #[error("Error deserializing JSON: {0}")]
    ErrorDeserializingJson(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
