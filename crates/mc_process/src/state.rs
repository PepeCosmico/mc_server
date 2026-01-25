use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerState {
    Stopped,
    Starting,
    Saving,
    CreatingBackup,
    Running,
    Stopping,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McVersion {
    pub mc_version: String,
    pub fabric_version: String,
}
