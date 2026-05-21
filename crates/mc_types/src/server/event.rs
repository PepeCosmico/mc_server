use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    Starting {
        mc_version: String,
        fabric_version: String,
    }, // Starting server
    Ready(String), // "Done (X.Xs)!"
    Stopping,      // "Stopping server"
    Chat {
        author: String,
        msg: String,
    }, // "<Jugador> mensaje"
    Unknown,       // Cualquier otra línea
}
