#![cfg(windows)]
use envlens_core::{
    discovery::discover,
    model::Status,
    python,
    runner::{run, success},
};
use std::{fs, path::Path, time::Duration};

#[test]
fn selected_venv_preserves_its_prefix_and_observes_a_real_pip_conflict() {
    let inv = discover(None);
    let python = inv
        .candidates
        .iter()
        .map(|c| Path::new(&c.path))
        .find(|p| {
            success(
                p,
                &["-I", "-c", "import venv,ensurepip,importlib.metadata"],
                8,
            )
            .is_ok()
        })
        .expect("This Windows integration test requires CPython 3.8+ with venv and ensurepip.");
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("独立 env");
    let setup = run(
        python,
        &["-I", "-m", "venv", dir.to_str().unwrap()],
        Duration::from_secs(60),
    )
    .unwrap();
    assert_eq!(setup.code, Some(0));
    let site = dir.join(r"Lib\site-packages");
    // A normal startup would exit here. The isolated probe must never load the fixture hook.
    fs::write(site.join("sitecustomize.py"), "raise SystemExit(88)\n").unwrap();
    let dist = site.join("envlens_fixture-1.0.dist-info");
    fs::create_dir(&dist).unwrap();
    fs::write(dist.join("METADATA"),"Metadata-Version: 2.1\nName: envlens-fixture\nVersion: 1.0\nRequires-Dist: envlens-missing-test-package>=2\n\n").unwrap();
    let exe = dir.join(r"Scripts\python.exe");
    let (observed, checks) = python::collect(Some(&exe));
    let env = observed.expect("selected venv should start in isolated mode");
    assert_eq!(env.kind, "venv");
    assert_eq!(Path::new(&env.prefix), dir);
    assert_eq!(Path::new(&env.executable), exe);
    let deps = checks.iter().find(|c| c.id == "python.deps").unwrap();
    assert_eq!(deps.status, Status::Warning);
    assert!(deps
        .evidence
        .iter()
        .any(|e| e.key == "pip_check" && e.value.contains("envlens-missing-test-package")));
}
#[test]
fn utf16_launcher_paths_preserve_unicode() {
    let line = " -V:3.14 * D:\\项目\\语言 模型\\python.exe\n";
    let mut data = vec![0xff, 0xfe];
    data.extend(line.encode_utf16().flat_map(u16::to_le_bytes));
    let matches = envlens_core::discovery::launcher_paths(&data);
    assert_eq!(matches.len(), 1);
    assert!(matches[0].0);
    assert_eq!(matches[0].1.to_str(), Some(r"D:\项目\语言 模型\python.exe"));
}
#[test]
fn rejects_network_and_device_paths_before_filesystem_access() {
    use envlens_core::runner::local_path;
    for path in [
        r"\\example.invalid\share\python.exe",
        r"\\.\device",
        r"C:relative.exe",
        r"\\?\UNC\server\share\python.exe",
    ] {
        assert!(!local_path(Path::new(path)));
    }
}
