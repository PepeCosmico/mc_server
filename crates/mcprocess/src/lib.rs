use serde::Serialize;

pub mod config;
pub mod error;
pub mod logs;
pub mod server;

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
