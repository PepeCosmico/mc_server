use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McVersion {
    pub mc_version: String,
    pub fabric_version: String,
}
