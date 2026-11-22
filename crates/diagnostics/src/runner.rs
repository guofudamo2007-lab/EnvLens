//! The only process boundary. Callers supply absolute executables and independent argv entries.
use command_group::CommandGroup;
use std::{
    env,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub const LIMIT: usize = 128 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    Missing,
    Denied,
    Timeout,
    OutputLimit,
    DriverNotLoaded,
    Invalid,
    Failed,
    Io,
}
impl Fault {
    pub fn code(self) -> &'static str {
        match self {
            Self::Missing => "not_found",
            Self::Denied => "permission_denied",
            Self::Timeout => "timeout",
            Self::OutputLimit => "output_limit",
            Self::DriverNotLoaded => "driver_not_loaded",
            Self::Invalid => "invalid_response",
            Self::Failed => "nonzero_exit",
            Self::Io => "io_error",
        }
    }
}
impl From<io::Error> for Fault {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::NotFound => Self::Missing,
            io::ErrorKind::PermissionDenied => Self::Denied,
            _ => Self::Io,
        }
    }
}
pub struct Output {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
}
fn drain(mut reader: impl Read, used: Arc<AtomicUsize>, keep: bool) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut block = [0u8; 4096];
    loop {
        let n = reader.read(&mut block)?;
        if n == 0 {
            return Ok(output);
        }
        let prev = used.fetch_add(n, Ordering::Relaxed);
        if prev.saturating_add(n) > LIMIT {
            return Ok(output);
        }
        if keep {
            output.extend_from_slice(&block[..n]);
        }
    }
}
pub fn run(exe: &Path, args: &[&str], timeout: Duration) -> Result<Output, Fault> {
    if !local_path(exe) {
        return Err(Fault::Invalid);
    }
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Avoid project imports, pip configuration and inherited credentials.
    cmd.env_clear();
    for key in [
        "SystemRoot",
        "WINDIR",
        "SystemDrive",
        "COMSPEC",
        "PATH",
        "PATHEXT",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "LOCALAPPDATA",
        "APPDATA",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
        "HOMEDRIVE",
        "HOMEPATH",
        "CUDA_PATH",
        "CUDA_VISIBLE_DEVICES",
        "CONDA_PREFIX",
        "VIRTUAL_ENV",
        "PYTHONNOUSERSITE",
    ] {
        if let Some(v) = env::var_os(key) {
            cmd.env(key, v);
        }
    }
    #[cfg(windows)]
    if let Some(dir) = env::var_os("SystemRoot") {
        cmd.current_dir(PathBuf::from(dir).join("System32"));
    }
    cmd.env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .env(
            "PIP_CONFIG_FILE",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("PIP_NO_INPUT", "1");
    let mut group = cmd.group();
    group.kill_on_drop(true);
    #[cfg(windows)]
    group.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let mut child = group.spawn()?;
    let stdout = child.inner().stdout.take().ok_or(Fault::Io)?;
    let stderr = child.inner().stderr.take().ok_or(Fault::Io)?;
    let used = Arc::new(AtomicUsize::new(0));
    let out_used = used.clone();
    let err_used = used.clone();
    let out = thread::spawn(move || drain(stdout, out_used, true));
    let err = thread::spawn(move || drain(stderr, err_used, false));
    let start = Instant::now();
    let result = loop {
        if used.load(Ordering::Relaxed) > LIMIT {
            break Err(Fault::OutputLimit);
        }
        if start.elapsed() >= timeout {
            break Err(Fault::Timeout);
        }
        match child.inner().try_wait() {
            Ok(Some(exit)) => break Ok(exit.code()),
            Ok(None) => thread::sleep(Duration::from_millis(15)),
            Err(e) => break Err(e.into()),
        }
    };
    // Close inherited pipes in descendants even when the immediate process exited normally.
    let _ = child.kill();
    let _ = child.inner().wait();
    let stdout = out.join().map_err(|_| Fault::Io)??;
    err.join().map_err(|_| Fault::Io)??;
    let code = result?;
    if used.load(Ordering::Relaxed) > LIMIT {
        return Err(Fault::OutputLimit);
    }
    Ok(Output { code, stdout })
}
pub fn success(exe: &Path, args: &[&str], seconds: u64) -> Result<Vec<u8>, Fault> {
    let out = run(exe, args, Duration::from_secs(seconds))?;
    match out.code {
        Some(0) => Ok(out.stdout),
        _ => Err(Fault::Failed),
    }
}
#[cfg(windows)]
pub fn local_path(path: &Path) -> bool {
    use std::path::{Component, Prefix};
    use windows_sys::Win32::{
        Storage::FileSystem::GetDriveTypeW,
        System::WindowsProgramming::{DRIVE_CDROM, DRIVE_FIXED, DRIVE_RAMDISK, DRIVE_REMOVABLE},
    };
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return false;
    };
    let letter = match prefix.kind() {
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
        _ => return false,
    };
    if !path.is_absolute() {
        return false;
    }
    let root = [letter as u16, b':' as u16, b'\\' as u16, 0];
    // SAFETY: root is a four-unit, null-terminated drive root.
    matches!(
        unsafe { GetDriveTypeW(root.as_ptr()) },
        DRIVE_FIXED | DRIVE_REMOVABLE | DRIVE_RAMDISK | DRIVE_CDROM
    )
}
#[cfg(not(windows))]
pub fn local_path(path: &Path) -> bool {
    path.is_absolute()
}
pub fn path_exes(name: &str) -> Vec<PathBuf> {
    let Some(paths) = env::var_os("PATH") else {
        return vec![];
    };
    env::split_paths(&paths)
        .filter(|d| local_path(d))
        .map(|d| d.join(name))
        .filter(|p| p.is_file())
        .collect()
}
pub fn system32(file: &str) -> Option<PathBuf> {
    env::var_os("SystemRoot").map(|p| PathBuf::from(p).join("System32").join(file))
}
