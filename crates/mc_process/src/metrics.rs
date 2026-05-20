use sysinfo::{Pid, ProcessesToUpdate, System};

/// Refresh and read CPU usage (%) and memory (bytes) for `pid`.
pub(crate) fn read(system: &mut System, pid: u32) -> Option<(f32, u64)> {
    let pid = Pid::from(pid as usize);
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system
        .process(pid)
        .map(|p| (p.cpu_usage(), p.memory()))
}
