use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("UX Template error")]
    ErrorPBTemplate(#[from] indicatif::style::TemplateError),
    #[error("Error trying to load config file")]
    ErrorLoadingConfigFile(#[from] config::ConfigError),
    #[error("Io Error connecting to daemon {0}")]
    ErrorIoConnectingToDaemon(#[from] std::io::Error),
    #[error("Timeout error connecting to daemon")]
    ErrorConnectingTimeout,
    #[error("Error deserializing JSON: {0}")]
    ErrorDeserializingJson(#[from] serde_json::Error),
    #[error("Error sending message: {0}")]
    ErrorSendingMessage(#[from] tokio_util::codec::LinesCodecError),
    #[error("Error building the logger: {0}")]
    ErrorBuildingLogger(#[from] tracing::subscriber::SetGlobalDefaultError),
}

pub type Result<T> = std::result::Result<T, Error>;
