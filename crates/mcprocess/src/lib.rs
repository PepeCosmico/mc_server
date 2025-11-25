use serde::Serialize;

pub mod config;
pub mod server;
pub mod error;
pub mod logs;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum ServerState {
    Stopped,
    Starting,
    Loading,
    Saving,
    CreatingBackup,
    Running,
    Stopping,
    Crashed,
}
