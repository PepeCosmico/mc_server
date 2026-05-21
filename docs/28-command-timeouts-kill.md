# Issue 28 — Command timeouts with automatic force-kill

There is no force-kill path today. `kill_signal: Arc<Notify>` is wired into the reaper task (`server.rs:308`) but **no public API ever calls `notify_*()` on it** — dead code. If the JVM hangs and ignores `/stop`, the daemon cannot recover.

## Scope

- Add `ServerProcess::force_stop()` that triggers `kill_signal` and awaits child exit.
- Every actor command (`Start`, `Stop`, `Backup`) carries a deadline. On timeout:
  - `Stop` → escalate to `force_stop()` automatically.
  - `Start` → escalate to `force_stop()` and report failure with the partial logs.
- `Stop` becomes fully idempotent: from `Stopped`/`Crashed` returns Ok immediately; from any other state, always converges to `Stopped` within the deadline.
- Timeouts come from config (`McConfig`), not hard-coded 60/120s like today (`actor.rs:46,84`).

**Depends on**: #17 (process module split makes this much cleaner).
