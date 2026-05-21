# Issue 31 — Integration tests for state machine edge cases

The state machine has effectively zero direct test coverage — every bug surfaces only manually. Add integration tests using `bin/mock_java.rs` covering the scenarios that currently break:

- Restart after crash (state was `Crashed`, expect to reach `Running` again).
- Crash during `Starting` (child exits before emitting `Done`).
- `Stop` while still `Starting`.
- `Backup` during `Running` (state cycles `Running → CreatingBackup → Running`; archive contains a quiesced snapshot — assert `Saved` was awaited).
- Force-stop when the JVM ignores `/stop` (extend `mock_java` with a mode that swallows the command).
- Daemon restart with orphan JVM (PID file recovery).

Each scenario should assert both the final `ServerState` and that no second child was spawned on top of an existing one.

**Depends on**: #17, state machine redesign, timeouts/kill, PID recovery.
