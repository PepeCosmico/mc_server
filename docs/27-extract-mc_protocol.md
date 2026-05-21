# Issue 27 — Extract mc_protocol crate

`mc_cli` currently depends on `mc_daemon` only to get the protocol types (`TcpRequest`, `TcpResponse`, `Op`, `ResponsePayload`, `StartData`). This drags the entire actor + TCP server into the CLI binary.

## Scope

- New crate `mc_protocol` containing the wire types currently in `mc_daemon/src/protocol.rs`.
- Dependencies kept minimal: `serde`, `serde_json`. (Open question: move `ServerState` here too, or keep it in `mc_process` and re-export — to decide during implementation.)
- `mc_daemon` depends on `mc_protocol`.
- `mc_cli` depends on `mc_protocol` instead of `mc_daemon`.

## Why

- Cleaner dependency graph.
- Smaller CLI binary.
- Enables a second interface (TUI, Axum/HTTP — #15) without coupling to the daemon crate.

Independent of the other robustness work — can land at any time.
