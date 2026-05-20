# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

This is a Cargo workspace (edition 2024). Two binaries: `mc_daemon` (long-running server manager) and `mc_cli` (`mc` command that talks to the daemon over TCP).

```bash
# Build everything
cargo build

# Run the daemon (host process that owns the Minecraft JVM)
cargo run -p mc_daemon

# Run the CLI against a running daemon
cargo run -p mc_cli -- start
cargo run -p mc_cli -- status
cargo run -p mc_cli -- stop
cargo run -p mc_cli -- op --player Steve --op true

# Tests (integration tests live in crates/mc_process/tests)
cargo test -p mc_process
cargo test -p mc_process --test integration_test test_start_version_detection

# Docker (production-style: builds mc_daemon, runs it under Temurin JRE 21)
docker compose up --build
```

The daemon and CLI both call `McConfig::new()` at startup, so the CLI must be run from a working directory where the same config layer resolves (see Configuration below). The CLI's `--address` flag (default `0.0.0.0:8080`) is currently **ignored** — the address comes from `cfg.client.get_addr()`. If you want a different daemon address, set it via config, not the flag.

## Architecture

The system is split across four crates that form a layered pipeline `CLI → TCP → Actor → ServerProcess → JVM`:

- **`mc_config`** — Layered config loader (defaults → OS config dir via `directories::ProjectDirs` → `config/{RUN_MODE}.toml` → `config/local.toml` → `APP__*` env vars). `RUN_MODE` defaults to `dev`. `McConfig` is shared by every crate; do not duplicate config structs elsewhere.
- **`mc_process`** — Owns the actual `java -jar` child process. `ServerProcess` exposes async ops (`start`, `stop`, `exec_command`, `op`, `get_metrics`) and three subscription channels: `state()` (watch), `version()` (watch), `logs()` (broadcast). It spawns two internal tasks per server lifetime: a **logger task** that parses stdout via `McLogParser` and drives state transitions, and a **reaper task** that awaits child exit and sets `Stopped`/`Crashed`. `bin/mock_java.rs` is the mock JVM used by integration tests — it emits Minecraft-shaped log lines so the parser/state machine can be exercised without a real server.
- **`mc_daemon`** — TCP server + actor. `tcp::server_loop` accepts connections, line-frames JSON (`tokio_util::codec::LinesCodec`) and translates each `TcpRequest` into a `DaemonCommand` sent over an mpsc channel. `actor::spawn_actor` is the **single owner** of the `ServerProcess`; all mutation goes through it. Long operations (Start/Stop) reply asynchronously: the actor returns immediately and a spawned task waits on `wait_for_state` with a timeout before sending the oneshot reply. This means new daemon commands must follow the same oneshot-reply pattern — never block the actor loop.
- **`mc_cli`** — Clap-based CLI. `commands::*::run` constructs a `TcpClient`, sends a typed `TcpRequest`, and renders the `TcpResponse`. The protocol types are re-used from `mc_daemon::protocol` — keep request/response shapes in sync there, not duplicated in the CLI.

### State machine (in `mc_process::state::ServerState`)

```
Stopped → Starting → Running
                       ↓
                    Stopping → Stopped
                       ↓
                    Crashed
```

Transitions are driven by parsed log events (`ServerEvent::Starting`, `Ready`, `Stopping`) in `ServerProcess::apply_state_transition`. `wait_for_state` in `mc_daemon/utils.rs` treats `Crashed` as a terminal failure (returns Err) **except** when waiting for `Stopped`, where `Crashed` is accepted as "no longer running."

### Log parsing

`McLogParser` expects Minecraft's `[HH:MM:SS] [thread/level]: msg` format. Unmatched lines become `ServerEvent::Unknown` with `level = "RAW"`. The Fabric-loader regex (`Loading Minecraft <ver> with Fabric Loader <ver>`) is how `McVersion` gets populated — if you target vanilla servers, you'll need to extend `detect_event`.

## Configuration

`config/dev.toml` and `config/prod.toml` are checked in. `config/local.toml` is git-ignored and overrides both. Env vars use the `APP__` prefix with `__` as the section separator: `APP__SERVER__PORT=…`, `APP__JAVA__XMX=4G`, etc. The `[client]` section configures **both** the daemon's bind address and the CLI's connect address — they share one socket pair.

## Conventions

- Strings in error messages, comments, and log output mix English and Spanish. Match the surrounding file when editing.
- `anyhow::Result` in the daemon binary boundary; `thiserror`-based crate-local `Error` types inside `mc_process` and `mc_cli`. Don't replace one with the other without reason.
- The actor uses `oneshot::Sender<Result<…, String>>` for replies — errors are stringified at the actor boundary because they cross the TCP wire. Inside `mc_process`, keep the typed `Error`.
- `kill_on_drop(true)` is set on the child; do not remove it without arranging an alternative shutdown path.
