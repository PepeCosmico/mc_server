use mcprocess::ServerState;
use tokio::sync::watch::Receiver;

pub async fn wait_for_state(mut state_rx: Receiver<ServerState>, state: ServerState) -> Result<(), String> {
    loop {
        let current = *state_rx.borrow();

        if current == state {
            return Ok(());
        }

        if matches!(current, ServerState::Crashed) {
            return Err("Server crashed".to_string());
        }

        if state_rx.changed().await.is_err() {
            return Err("Internal error".to_string());
        }
    }
}