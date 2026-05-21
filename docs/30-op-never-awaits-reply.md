# Issue 30 — Op command never awaits actor reply

`mc_daemon/src/tcp.rs:108-114`:

```rust
TcpRequest::Op(op) => {
    let (reply_tx, _reply_rx) = oneshot::channel();   // <-- _reply_rx dropped
    tx.send(DaemonCommand::Op(reply_tx, op.clone())).await.ok();
    Ok(serde_json::to_string(&TcpResponse::ok_with(...))?)  // always "success"
}
```

The handler responds `success: true` echoing the request, regardless of whether the actor succeeded. The actor's reply is sent into a dropped oneshot.

**Fix**: await `reply_rx` like `Start`/`Stop` do; map `Ok(())` → `TcpResponse::ok_with(ResponsePayload::OpResult(op))`, `Err(e)` → `TcpResponse::error(&e)`.

---

*(Backup-related bug from the original scope of this issue was dropped: backup functionality has been removed entirely; it will be reintroduced later as part of a new issue.)*
