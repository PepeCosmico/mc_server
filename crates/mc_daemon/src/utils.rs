use mc_types::server::state::ServerState;
use tokio::sync::watch::Receiver;

pub async fn wait_for_state(
    mut state_rx: Receiver<ServerState>,
    target: ServerState,
) -> Result<(), String> {
    loop {
        let current = *state_rx.borrow();

        if current == target {
            return Ok(());
        }

        match target {
            ServerState::Stopped => {
                if current == ServerState::Crashed {
                    return Ok(());
                }
            }

            _ => {}
        }

        if current == ServerState::Crashed {
            return Err("Server crashed".to_string());
        }

        if state_rx.changed().await.is_err() {
            return Err("Internal error: State channel closed".to_string());
        }
    }
}
