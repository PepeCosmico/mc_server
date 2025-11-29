use serde::{Deserialize, Serialize};

// Lo que recibimos (JSON)
#[derive(Debug, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum TcpRequest {
    Start,
    Stop,
    Status,
}

// Lo que respondemos (JSON)
#[derive(Debug, Serialize)]
pub struct TcpResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

pub struct TcpResponseBuilder<T> {
    success: bool,
    message: Option<String>,
    data: Option<T>,
}

impl<T> TcpResponseBuilder<T> {
    pub fn builder(success: bool) -> Self {
        Self {
            success,
            message: None,
            data: None,
        }
    }

    pub fn message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    pub fn data(mut self, data: T) -> Self {
        self.data = Some(data);
        self
    }

    pub fn build(self) -> TcpResponse<T> {
        TcpResponse {
            success: self.success,
            message: self.message,
            data: self.data,
        }
    }
}
