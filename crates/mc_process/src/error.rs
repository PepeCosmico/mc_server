use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Error trying to create working directory")]
    CreateWorkingDirectoryFailed(#[source] std::io::Error),

    #[error("Error trying to write eula file")]
    CreateEulaFileFailed(#[source] std::io::Error),

    #[error("Error trying resolve working_dir absolute path")]
    ResolveWorkingDirectoryFailed(#[source] std::io::Error),

    #[error("Error trying to write with ChildStdin")]
    WriteStdinFailed(#[source] std::io::Error),

    #[error("Error trying to execute command when the server is not runnning")]
    WriteWhileNotRunningError,

    #[error("JAR file does not exist")]
    JarFileDoesNotExist,

    #[error("Failed to spawn java process")]
    SpawnFailed(#[source] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
