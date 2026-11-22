use super::*;
fn sample(n: usize) -> Report {
    Report {
        schema_version: 1,
        id: format!("fixture-{n:02}"),
        started_at: format!("2026-01-01T00:00:{n:02}Z"),
        duration_ms: 5,
        interpreter: Some(r"C:\Users\Private\python.exe".into()),
        project: None,
        redacted: false,
        checks: vec![],
    }
}
#[test]
fn persistence_is_atomic_and_always_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let r = sample(1);
    save(tmp.path(), &r).unwrap();
    assert!(save(tmp.path(), &r).is_err());
    let h = load(tmp.path()).unwrap();
    assert_eq!(h.reports.len(), 1);
    assert!(h.reports[0].redacted);
    assert_eq!(h.reports[0].interpreter.as_deref(), Some("<python.exe>"));
    assert_eq!(h.unreadable, 0);
}
#[test]
fn retention_keeps_newest_and_reports_corruption() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("broken.json"), "{").unwrap();
    for n in 0..22 {
        save(tmp.path(), &sample(n)).unwrap();
    }
    let archive = load(tmp.path()).unwrap();
    assert_eq!(archive.unreadable, 1);
    assert_eq!(archive.reports.len(), 20);
    assert_eq!(archive.reports[0].id, "fixture-21");
    assert_eq!(archive.reports.last().unwrap().id, "fixture-02");
    assert!(tmp.path().join("broken.json").exists());
}
