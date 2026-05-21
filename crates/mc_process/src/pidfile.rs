use crate::error::{Error, Result};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn path(working_dir: &Path) -> PathBuf {
    working_dir.join("runtime").join("server.pid")
}

pub fn write(working_dir: &Path, pid: u32) -> Result<()> {
    let pidfile_path = path(working_dir);
    let dir = pidfile_path.parent().unwrap();
    fs::create_dir_all(dir).map_err(Error::CreateRuntimeDirectoryFailed)?;

    let tmp_path = dir.join("server.pid.tmp");
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp_path).map_err(Error::CreatePidFileFailed)?;
    writeln!(f, "{}", pid).map_err(Error::CreatePidFileFailed)?;
    f.sync_all().map_err(Error::CreatePidFileFailed)?;
    drop(f);

    fs::rename(&tmp_path, &pidfile_path).map_err(Error::CreatePidFileFailed)?;
    Ok(())
}

pub fn read(working_dir: &Path) -> Result<Option<u32>> {
    let pidfile_path = path(working_dir);

    let content = match fs::read_to_string(&pidfile_path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::ReadPidFileFailed(e)),
    };

    let pid: u32 = content
        .trim()
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        .map_err(Error::ReadPidFileFailed)?;

    Ok(Some(pid))
}

pub fn remove(working_dir: &Path) -> Result<()> {
    let pidfile_path = path(working_dir);
    match fs::remove_file(pidfile_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::RemovePidFileFailed(e)),
    }
}
