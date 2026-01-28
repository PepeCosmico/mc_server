use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

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

impl Display for ServerState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerState::Stopped => write!(f, "Stopped"),
            ServerState::Starting => write!(f, "Starting"),
            ServerState::Saving => write!(f, "Saving"),
            ServerState::CreatingBackup => write!(f, "CreatingBackup"),
            ServerState::Running => write!(f, "Running"),
            ServerState::Stopping => write!(f, "Stopping"),
            ServerState::Crashed => write!(f, "Crashed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McVersion {
    pub mc_version: String,
    pub fabric_version: String,
}
