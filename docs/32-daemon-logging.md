# Issue 32 — Basic structured logging in mc_daemon

Today `mc_daemon` uses `println!/eprintln!` everywhere. `tracing` and
`tracing-subscriber` are already in `Cargo.toml` but never initialized. There
is no level filter, no target, no timestamp.

## Scope

- Initialize `tracing_subscriber::fmt` in `mc_daemon/src/main.rs` before
  `recovery::run`. Format: timestamp + level + target + message. Default level
  `info`. Drive the filter from `RUST_LOG` via the `env-filter` feature.
- Replace every `println!/eprintln!` in `mc_daemon` (`main.rs`, `tcp.rs`,
  `actor.rs`, `recovery.rs`) with the matching `tracing` macro:
  - `info!` for normal lifecycle events ("daemon stopped", "killing orphan pid X").
  - `warn!` for recoverable degradations ("pidfile unreadable, assuming clean start").
  - `error!` for failure conditions returned to the caller.
  - `debug!` for the recovery diagnostic chatter
    (`classify: pid X cwd = ...`, `kill: pid X still alive after 5s`, etc.).
- Demote the `recovery` step-by-step logs from `eprintln!` to `debug!` so they
  only appear under `RUST_LOG=mc_daemon::recovery=debug`. Keep the high-level
  outcomes (`killing orphan`, `orphan killed`) at `info!`.

## Out of scope

- File rotation, JSON output, OTel/journald exporters. Add them in a follow-up
  if and when actually needed.
- `mc_cli` output formatting — it has its own UX layer (`colored`, `indicatif`)
  and does not need structured logs.
- `mc_process` logging — its `McLog` broadcast channel already carries the
  Minecraft JVM logs as typed events; consumers decide how to render them.

Independent of the other robustness work — can land any time.
