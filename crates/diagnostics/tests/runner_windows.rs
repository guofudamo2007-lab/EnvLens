#![cfg(windows)]
use envlens_core::runner::{run, system32, Fault, LIMIT};
use std::{
    path::Path,
    time::{Duration, Instant},
};
fn ps() -> std::path::PathBuf {
    system32(r"WindowsPowerShell\v1.0\powershell.exe").unwrap()
}
#[test]
fn missing_binary_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        run(&dir.path().join("absent.exe"), &[], Duration::from_secs(2)),
        Err(Fault::Missing)
    ));
}
#[test]
fn directories_are_denied_by_createprocess() {
    let d = tempfile::tempdir().unwrap();
    assert!(matches!(
        run(d.path(), &[], Duration::from_secs(2)),
        Err(Fault::Denied)
    ));
}
#[test]
fn relative_program_is_rejected() {
    assert!(matches!(
        run(Path::new("python.exe"), &[], Duration::from_secs(2)),
        Err(Fault::Invalid)
    ));
}
#[test]
fn timeout_terminates_the_job() {
    let start = Instant::now();
    let r = run(
        &ps(),
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 20",
        ],
        Duration::from_millis(450),
    );
    assert!(matches!(r, Err(Fault::Timeout)));
    assert!(start.elapsed() < Duration::from_secs(7));
}
#[test]
fn oversized_stdout_is_bounded() {
    let r = run(
        &ps(),
        &["-NoProfile", "-Command", "[Console]::Write(('a' * 300000))"],
        Duration::from_secs(8),
    );
    assert!(matches!(r, Err(Fault::OutputLimit)));
}
#[test]
fn oversized_stderr_is_bounded_too() {
    let r = run(
        &ps(),
        &[
            "-NoProfile",
            "-Command",
            "[Console]::Error.Write(('a' * 300000))",
        ],
        Duration::from_secs(8),
    );
    assert!(matches!(r, Err(Fault::OutputLimit)));
}
#[test]
fn nonzero_exit_is_not_success() {
    let r = run(
        &ps(),
        &["-NoProfile", "-Command", "exit 7"],
        Duration::from_secs(8),
    )
    .unwrap();
    assert_eq!(r.code, Some(7));
    assert!(r.stdout.len() <= LIMIT);
}
#[test]
fn arguments_with_shell_punctuation_remain_one_literal() {
    let dir = tempfile::tempdir().unwrap();
    let literal = "a b & (two); $name [literal]";
    let file = dir.path().join(literal);
    std::fs::write(&file, []).unwrap();
    let out = run(
        &system32("where.exe").unwrap(),
        &["/r", dir.path().to_str().unwrap(), literal],
        Duration::from_secs(8),
    )
    .unwrap();
    assert_eq!(out.code, Some(0));
    assert!(String::from_utf8_lossy(&out.stdout)
        .trim()
        .ends_with(literal));
}
