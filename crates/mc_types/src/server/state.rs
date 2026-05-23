use crate::server::event::ServerEvent;
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
    /// The only event-driven transition is `Starting → Running` on `Ready`.
    /// Everything else is driven imperatively: `start()` sets `Starting`,
    /// `stop()` sets `Stopping`, and the reaper sets `Stopped`/`Crashed` when
    /// the child exits.
    pub fn next(self, event: &ServerEvent) -> Option<Self> {
        match (self, event) {
            (ServerState::Starting, ServerEvent::Ready(_)) => Some(ServerState::Running),
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
    fn starting_to_running_on_ready() {
        assert_eq!(
            ServerState::Starting.next(&ServerEvent::Ready("1.0s".into())),
            Some(ServerState::Running)
        );
    }

    #[test]
    fn ready_is_ignored_outside_starting() {
        let evt = ServerEvent::Ready("1.0s".into());
        assert_eq!(ServerState::Stopped.next(&evt), None);
        assert_eq!(ServerState::Running.next(&evt), None);
        assert_eq!(ServerState::Stopping.next(&evt), None);
        assert_eq!(ServerState::Crashed.next(&evt), None);
    }

    #[test]
    fn starting_event_never_transitions() {
        // Starting is now imperative (set by `start()`), not log-driven.
        for s in [
            ServerState::Stopped,
            ServerState::Starting,
            ServerState::Running,
            ServerState::Stopping,
            ServerState::Crashed,
        ] {
            assert_eq!(s.next(&starting_evt()), None);
        }
    }

    #[test]
    fn stopping_event_never_transitions() {
        // Stopping is now imperative (set by `stop()`), not log-driven.
        for s in [
            ServerState::Stopped,
            ServerState::Starting,
            ServerState::Running,
            ServerState::Stopping,
            ServerState::Crashed,
        ] {
            assert_eq!(s.next(&ServerEvent::Stopping), None);
        }
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
