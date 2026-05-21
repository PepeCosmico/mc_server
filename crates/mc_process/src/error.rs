use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Error trying to create working directory")]
    CreateWorkingDirectoryFailed(#[source] std::io::Error),

    #[error("Error trying to create runtime directory")]
    CreateRuntimeDirectoryFailed(#[source] std::io::Error),

    #[error("Error trying to write eula file")]
    CreateEulaFileFailed(#[source] std::io::Error),

    #[error("Error trying to write pid file")]
    CreatePidFileFailed(#[source] std::io::Error),

    #[error("Error trying to read pid file")]
    ReadPidFileFailed(#[source] std::io::Error),

    #[error("Error trying to remove pid file")]
    RemovePidFileFailed(#[source] std::io::Error),

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
