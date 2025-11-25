use serde::{Deserialize, Serialize};

// Lo que recibimos (JSON)
#[derive(Debug, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum TcpRequest {
    Start,
    Stop,
    Kill,
    Status,
    Input(String),
    Backup,
}

// Lo que respondemos (JSON)
#[derive(Debug, Serialize)]
pub struct TcpResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> TcpResponse<T> {
    pub fn success(msg: String) -> Self {
        Self { success: true, message: Some(msg), data: None }
    }

    // Función útil para enviar datos (ej: Status o Filename)
    pub fn data(data: T) -> Self {
        Self { success: true, message: None, data: Some(data) }
    }

    pub fn error(msg: String) -> Self {
        Self { success: false, message: Some(msg), data: None }
    }
}