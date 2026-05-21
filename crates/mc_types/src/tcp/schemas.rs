use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Op {
    pub player: String,
    pub op: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartData {
    pub version: String,
    pub address: String,
}
