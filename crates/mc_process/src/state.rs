use crate::logs::ServerEvent;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Crashed,
}

impl ServerState {
    /// Pure state-machine transition.
    ///
    /// Returns the next state if `event` triggers a transition from `self`,
    /// otherwise `None`. No I/O, no allocation — safe to unit-test without a
    /// JVM.
    pub fn next(self, event: &ServerEvent) -> Option<Self> {
        match (self, event) {
            (ServerState::Stopped, ServerEvent::Starting { .. }) => Some(ServerState::Starting),
            (ServerState::Starting, ServerEvent::Ready(_)) => Some(ServerState::Running),
            (ServerState::Running, ServerEvent::Stopping) => Some(ServerState::Stopping),
            _ => None,
        }
    }
}

impl Display for ServerState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerState::Stopped => write!(f, "Stopped"),
            ServerState::Starting => write!(f, "Starting"),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn starting_evt() -> ServerEvent {
        ServerEvent::Starting {
            mc_version: "1.21.4".to_string(),
            fabric_version: "0.17.3".to_string(),
        }
    }

    #[test]
    fn stopped_to_starting() {
        assert_eq!(
            ServerState::Stopped.next(&starting_evt()),
            Some(ServerState::Starting)
        );
    }

    #[test]
    fn starting_to_running_on_ready() {
        assert_eq!(
            ServerState::Starting.next(&ServerEvent::Ready("1.0s".into())),
            Some(ServerState::Running)
        );
    }

    #[test]
    fn running_to_stopping() {
        assert_eq!(
            ServerState::Running.next(&ServerEvent::Stopping),
            Some(ServerState::Stopping)
        );
    }

    #[test]
    fn crashed_does_not_restart_on_starting_event() {
        // Documenting current behaviour: restart-after-crash is broken; the
        // state machine redesign (issue #26) will lift this restriction.
        assert_eq!(ServerState::Crashed.next(&starting_evt()), None);
    }

    #[test]
    fn ready_is_ignored_outside_starting() {
        let evt = ServerEvent::Ready("1.0s".into());
        assert_eq!(ServerState::Stopped.next(&evt), None);
        assert_eq!(ServerState::Running.next(&evt), None);
        assert_eq!(ServerState::Stopping.next(&evt), None);
    }

    #[test]
    fn stopping_is_ignored_outside_running() {
        assert_eq!(ServerState::Stopped.next(&ServerEvent::Stopping), None);
        assert_eq!(ServerState::Starting.next(&ServerEvent::Stopping), None);
    }

    #[test]
    fn chat_and_unknown_never_transition() {
        let chat = ServerEvent::Chat {
            author: "Steve".into(),
            msg: "hi".into(),
        };
        for s in [
            ServerState::Stopped,
            ServerState::Starting,
            ServerState::Running,
            ServerState::Stopping,
            ServerState::Crashed,
        ] {
            assert_eq!(s.next(&chat), None);
            assert_eq!(s.next(&ServerEvent::Unknown), None);
        }
    }
}
