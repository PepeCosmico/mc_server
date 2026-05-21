use mc_types::server::event::ServerEvent;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McLog {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub event: ServerEvent,
}

impl McLog {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

pub struct McLogParser;

impl McLogParser {
    /// Parsea una línea cruda y devuelve un LogEntry estructurado
    pub fn parse(line: &str) -> Option<McLog> {
        // Regex principal: [Hora] [Hilo/Nivel]: Mensaje
        static LOG_RE: OnceLock<Regex> = OnceLock::new();
        let re = LOG_RE
            .get_or_init(|| Regex::new(r"^\[(\d{2}:\d{2}:\d{2})] \[([^]]+)]: (.*)$").unwrap());

        if let Some(caps) = re.captures(line) {
            let timestamp = caps.get(1)?.as_str().to_string();
            let level = caps.get(2)?.as_str().to_string();
            let raw_msg = caps.get(3)?.as_str().to_string();

            // Detectamos qué evento es
            let event = Self::detect_event(&level, &raw_msg);

            Some(McLog {
                timestamp,
                level,
                message: raw_msg,
                event,
            })
        } else {
            // Si la línea no tiene formato estándar (stacktraces, stderr, etc.)
            None
        }
    }

    /// Lógica interna para clasificar el mensaje
    fn detect_event(level: &str, msg: &str) -> ServerEvent {
        static DONE_RE: OnceLock<Regex> = OnceLock::new();
        static FABRIC_START_RE: OnceLock<Regex> = OnceLock::new();
        static CHAT_RE: OnceLock<Regex> = OnceLock::new();

        let is_server_thread = level.starts_with("Server thread");
        let is_main_thread = level.starts_with("main");
        let is_async_chat_thread = level.starts_with("Async Chat Thread");

        // Chat detection runs first so that player messages whose body happens
        // to look like a system command (e.g. "Stopping the server") never get
        // misclassified as a system event.
        if is_async_chat_thread {
            let chat_re = CHAT_RE.get_or_init(|| Regex::new(r"^<([^>]+)> (.*)$").unwrap());
            if let Some(caps) = chat_re.captures(msg) {
                return ServerEvent::Chat {
                    author: caps[1].to_string(),
                    msg: caps[2].to_string(),
                };
            }
        }

        if is_main_thread {
            let start_re = FABRIC_START_RE.get_or_init(|| {
                Regex::new(r"^Loading Minecraft (\S+) with Fabric Loader (\S+)").unwrap()
            });

            if let Some(caps) = start_re.captures(msg) {
                return ServerEvent::Starting {
                    mc_version: caps[1].to_string(),
                    fabric_version: caps[2].to_string(),
                };
            }
        }

        if is_server_thread {
            let done_re = DONE_RE.get_or_init(|| Regex::new(r"^Done \((.+)\)!").unwrap());
            if let Some(caps) = done_re.captures(msg) {
                return ServerEvent::Ready(caps[1].to_string());
            }

            if msg.starts_with("Stopping the server") {
                return ServerEvent::Stopping;
            }
        }

        ServerEvent::Unknown
    }
}
