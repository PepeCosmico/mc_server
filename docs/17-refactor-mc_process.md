# Issue 17 — Refactor mc_process internal structure

Reorganize the internal structure of `mc_process` without changing its public API. **Starting point** of the broader robustness refactor (see #26, #27, #28, #29, #30, #31).

## Scope

- Split `server.rs` (~430 LOC) into submodules:
  - `process.rs` — spawn, reaper, stdin, kill
  - `backup.rs` — save-off/save-all/save-on + tar.gz archive
  - `metrics.rs` — sysinfo wrapper
  - `server.rs` — thin facade composing the above
- Move state transition logic out of `server.rs::apply_state_transition` into a **pure** `impl ServerState { fn next(self, event: &ServerEvent) -> Option<Self> }` in `state.rs`. Unit-testable without spawning a JVM — currently transitions only exercise via `mock_java` integration tests.
- Remove dead variants from `error.rs` (`ReadConfigFailed`, `DeserConfigFailed`) — `mc_process` receives `McConfig` already constructed.
- Emit typed `McLog` over the `log_tx` broadcast channel instead of pre-serialized JSON (`McLog::to_json` becomes a consumer concern; daemon serializes when shipping over TCP).
- Replace `saved_signal: Arc<Notify>` with a per-operation `oneshot` created inside `backup()` — the current shared `Notify` pattern is fragile (see #30).

## Out of scope (separate issues)
State-machine redesign (#26), protocol extraction (#27), command timeouts (#28), PID recovery (#29), bug fixes (#30), tests (#31).
