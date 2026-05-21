# Issue 29 — PID file + orphan JVM recovery on daemon restart

If the daemon dies abruptly (`kill -9`, OOM, host reboot), the JVM child is orphaned and `kill_on_drop` never fires. Port 25565 stays bound and the next daemon start fails opaquely.

## Scope

- Write `runtime/server.pid` on JVM spawn; remove it on clean exit.
- On daemon startup, if pid file exists:
  - Use `sysinfo` (already a dependency) to check whether the PID is alive and belongs to a `java` process started from the configured `working_dir`.
  - Decide policy (to discuss during implementation):
    - **(a)** Refuse to start until manually cleared.
    - **(b)** Auto-kill the orphan and proceed.
    - **(c)** Adopt the running JVM (would require recovering its stdin — likely not feasible, mention for completeness).
- On clean daemon shutdown (Ctrl+C in `tcp::server_loop`), stop the JVM gracefully before exiting. Today the runtime drops abruptly, only `kill_on_drop` saves us.
