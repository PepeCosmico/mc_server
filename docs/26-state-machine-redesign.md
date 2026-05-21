# Issue 26 — Redesign state machine: Child as ground truth + healthcheck fallback

Make `ServerState` derive from a single source of truth (the `Child` handle) instead of log-regex parsing. Today state desynchronizes from reality in multiple ways and the daemon ends up in unrecoverable states.

## Goals

- The `Child` handle is the ground truth:
  - `Stopped` ⇔ no child
  - `Starting` / `Running` / `Stopping` ⇔ child alive
  - `Crashed` ⇔ child exited non-zero
- Log parsing only decides **when** `Starting → Running` (readiness), nothing else.
- **Fallback readiness**: if no readiness log event arrives within a soft timeout, perform a TCP healthcheck against `127.0.0.1:25565`. Unblocks vanilla servers and unknown modpacks where the current Fabric-only regex never fires (`logs.rs:77`).
- Allow restart from any terminal state (`Stopped`, `Crashed`). Today `_ → Starting` is gated on `current == Stopped` (`server.rs:398`), so restart after a crash hangs forever waiting for `Running`.
- `Stopping` must always converge to `Stopped` within its deadline (see timeouts issue).
- Sub-states (`Saving`, `CreatingBackup`) become orthogonal flags, not main states.

**Depends on**: #17 (pure transition function makes this redesign safe to land).
