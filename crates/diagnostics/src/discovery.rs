use crate::runner::{local_path as local, path_exes, success};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub path: String,
    pub sources: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub candidates: Vec<Candidate>,
    pub project: Option<String>,
    pub launcher: Option<String>,
    pub notices: Vec<String>,
}
pub fn python_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let Some(stem) = name
        .strip_suffix(".exe")
        .and_then(|s| s.strip_prefix("python"))
    else {
        return false;
    };
    local(path)
        && (stem.is_empty()
            || (stem.starts_with('3') && stem.bytes().all(|b| b.is_ascii_digit() || b == b'.')))
        && !path
            .to_string_lossy()
            .to_ascii_lowercase()
            .contains(r"\microsoft\windowsapps\")
        && path.is_file()
}
pub fn same(a: &str, b: &str) -> bool {
    a.replace('/', r"\")
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(b.replace('/', r"\").trim_end_matches('\\'))
}
impl Inventory {
    pub fn add(&mut self, path: &Path, source: &str) {
        if !python_path(path) {
            return;
        }
        let path = path.to_string_lossy().into_owned();
        if let Some(c) = self.candidates.iter_mut().find(|c| same(&c.path, &path)) {
            if !c.sources.iter().any(|s| s == source) {
                c.sources.push(source.into())
            }
        } else if self.candidates.len() < 64 {
            self.candidates.push(Candidate {
                path,
                sources: vec![source.into()],
            });
        }
    }
}
fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).map(PathBuf::from)
}
fn launcher() -> Option<PathBuf> {
    let known = [
        env_path("SystemRoot").map(|dir| dir.join("py.exe")),
        env_path("LOCALAPPDATA").map(|dir| dir.join(r"Programs\Python\Launcher\py.exe")),
    ];
    known
        .into_iter()
        .flatten()
        .find(|p| local(p) && p.is_file())
        .or_else(|| {
            path_exes("py.exe")
                .into_iter()
                .find(|p| local(p) && p.is_file())
        })
}
fn launcher_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.get(1) == Some(&0) {
        let words: Vec<_> = bytes
            .strip_prefix(&[0xff, 0xfe])
            .unwrap_or(bytes)
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_le_bytes(*p))
            .collect();
        return String::from_utf16_lossy(&words);
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.into();
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_ACP};
        let mut wide = vec![0u16; bytes.len()];
        // SAFETY: the bounded input and output buffers outlive the conversion call.
        let n = unsafe {
            MultiByteToWideChar(
                CP_ACP,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                wide.as_mut_ptr(),
                wide.len() as i32,
            )
        };
        if n > 0 {
            return String::from_utf16_lossy(&wide[..n as usize]);
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}
pub fn launcher_paths(bytes: &[u8]) -> Vec<(bool, PathBuf)> {
    let mut paths = vec![];
    let text = launcher_text(bytes);
    for line in text.lines().take(128) {
        let Some(i) = line.as_bytes().windows(3).position(|p| {
            p[0].is_ascii_alphabetic() && p[1] == b':' && matches!(p[2], b'\\' | b'/')
        }) else {
            continue;
        };
        let path = line[i..].trim_end_matches([' ', '*', '\t', '"']).trim_end();
        if path.to_ascii_lowercase().ends_with(".exe") {
            paths.push((
                line[..i].contains('*') || line.trim_end().ends_with('*'),
                PathBuf::from(path),
            ));
        }
    }
    paths.sort_by_key(|p| !p.0);
    paths
}
pub fn discover(project: Option<&Path>) -> Inventory {
    let mut inv = Inventory::default();
    if let Some(dir) = project {
        inv.project = Some(dir.to_string_lossy().into());
        if !local(dir) {
            inv.notices.push("local_only".into());
            return inv;
        }
        for name in [
            ".venv",
            "venv",
            "env",
            "",
            "python_embeded",
            "python_embedded",
        ] {
            let root = dir.join(name);
            inv.add(&root.join(r"Scripts\python.exe"), "project");
            inv.add(&root.join("python.exe"), "project");
        }
        if inv.candidates.is_empty() {
            inv.notices.push("project_empty".into());
        }
        return inv;
    }
    for (key, file) in [
        ("VIRTUAL_ENV", r"Scripts\python.exe"),
        ("CONDA_PREFIX", "python.exe"),
    ] {
        if let Some(dir) = env_path(key) {
            let p = dir.join(file);
            if !python_path(&p) {
                inv.notices.push(format!("stale_{key}"));
            }
            inv.add(&p, key);
        }
    }
    for exe in ["python.exe", "python3.exe"] {
        for p in path_exes(exe) {
            inv.add(&p, "PATH");
        }
    }
    if let Some(py) = launcher() {
        inv.launcher = Some(py.to_string_lossy().into());
        match success(&py, &["-0p"], 6) {
            Ok(bytes) => {
                for (_, p) in launcher_paths(&bytes) {
                    inv.add(&p, "launcher");
                }
            }
            Err(e) => inv.notices.push(format!("launcher_{}", e.code())),
        }
    }
    if let Some(home) = env_path("USERPROFILE").filter(|p| local(p)) {
        let path = home.join(r".conda\environments.txt");
        match File::open(path) {
            Ok(file) => {
                let mut data = String::new();
                if file.take(65537).read_to_string(&mut data).is_ok() && data.len() <= 65536 {
                    for line in data.lines().take(128) {
                        let root = Path::new(line.trim());
                        if local(root) {
                            inv.add(&root.join("python.exe"), "conda");
                        }
                    }
                } else {
                    inv.notices.push("conda_registry_unreadable".into());
                }
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                inv.notices.push("conda_registry_unreadable".into())
            }
            Err(_) => {}
        }
    }
    inv
}
