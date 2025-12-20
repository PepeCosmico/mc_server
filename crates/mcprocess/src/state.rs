use serde::Serialize;

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
