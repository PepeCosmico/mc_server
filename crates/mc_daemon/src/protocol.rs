use mc_process::state::ServerState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "action", content = "payload")]
pub enum TcpRequest {
    Start,
    Stop,
    Status,
    Op(Op),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Op {
    pub player: String,
    pub op: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TcpResponse {
    pub success: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    #[serde(skip_serializing_if = "is_payload_none")]
    pub data: ResponsePayload,
}

impl TcpResponse {
    pub fn ok() -> Self {
        Self {
            success: true,
            error: None,
            data: ResponsePayload::None,
        }
    }

    pub fn ok_with(payload: ResponsePayload) -> Self {
        Self {
            success: true,
            error: None,
            data: payload,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            error: Some(msg.to_string()),
            data: ResponsePayload::None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponsePayload {
    Status(ServerState),
    Start(StartData),
    OpResult(Op),
    Simple(String),
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartData {
    pub version: String,
    pub address: String,
}

fn is_payload_none(payload: &ResponsePayload) -> bool {
    matches!(payload, ResponsePayload::None)
}
