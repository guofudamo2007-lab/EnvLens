#![cfg(windows)]
use envlens_core::{
    discovery::{discover, python_path, Inventory},
    model::Status,
    python,
};
use std::{fs, path::Path};
#[test]
fn project_candidates_are_scoped_and_deduplicated() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = tmp.path().join(r".venv\Scripts\python.exe");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, []).unwrap();
    let inv = discover(Some(tmp.path()));
    assert_eq!(inv.candidates.len(), 1);
    assert_eq!(Path::new(&inv.candidates[0].path), exe);
    let mut inv = Inventory::default();
    inv.add(&exe, "PATH");
    inv.add(&exe, "launcher");
    assert_eq!(inv.candidates[0].sources, ["PATH", "launcher"]);
    let other = tmp.path().join("unrelated.exe");
    fs::write(&other, []).unwrap();
    assert!(!python_path(&other));
}
#[test]
fn empty_project_never_falls_back_to_global() {
    let tmp = tempfile::tempdir().unwrap();
    let inv = discover(Some(tmp.path()));
    assert!(inv.candidates.is_empty());
    assert_eq!(python::discovery_check(&inv).status, Status::Warning);
}
#[test]
fn conda_project_prefix_is_a_valid_candidate() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("python.exe");
    fs::create_dir(tmp.path().join("conda-meta")).unwrap();
    fs::write(p, []).unwrap();
    let inv = discover(Some(tmp.path()));
    assert_eq!(inv.candidates.len(), 1);
    assert_eq!(inv.candidates[0].sources, ["project"]);
}
#[test]
fn missing_interpreter_reports_unknown_and_skips_downstream() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, c) = python::collect(Some(&tmp.path().join("gone.exe")));
    assert_eq!(c[0].status, Status::Unknown);
    assert!(c.iter().skip(1).all(|c| c.status == Status::Skipped));
}
