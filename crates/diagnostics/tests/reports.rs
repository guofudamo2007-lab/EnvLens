use envlens_core::{model::*, report::*};
#[test]
fn recognizable_credentials_are_scrubbed_even_for_private_exports() {
    let mut r = fixture();
    let sample = format!("api_key={}", "private-fixture");
    r.checks[0].evidence[0].value = sample;
    scrub_credentials(&mut r);
    let exported = render(&r, Format::Json, false, true);
    assert!(exported.contains("[REDACTED]"));
    assert!(!exported.contains("private-fixture"));
}
fn fixture() -> Report {
    let path = r"C:\Users\O'Brien\Private Project\python.exe";
    let c = Check::new(
        "test",
        Group::Python,
        ("异常", "Failure"),
        Status::Warning,
        ("测试", "Test"),
    )
    .private("sys.executable", path)
    .private("model", "private/team-model")
    .fact("driver", "<tag attr=\"value\">")
    .explain(
        Confidence::Confirmed,
        ("影响", "Impact"),
        ("原因", "Cause"),
        &[("核对", "Verify")],
        &[PY_DOC],
    )
    .command(ps_python(path, "-m pip check"));
    Report {
        schema_version: 1,
        id: "fixture".into(),
        started_at: "2026-01-01T00:00:00Z".into(),
        duration_ms: 1000,
        interpreter: Some(path.into()),
        project: Some(r"D:\Private Project".into()),
        redacted: false,
        checks: vec![c],
    }
}
#[test]
fn all_formats_remove_private_fields_and_quoted_paths() {
    let r = fixture();
    for f in [
        Format::Json,
        Format::Markdown,
        Format::Html,
        Format::Summary,
    ] {
        let s = render(&r, f, true, true);
        assert!(!s.contains("Brien"));
        assert!(!s.contains("Private Project"));
        assert!(!s.contains("team-model"));
    }
    assert!(!r.redacted);
}
#[test]
fn redaction_is_idempotent() {
    let r = fixture();
    assert_eq!(
        render(&redact(&r), Format::Json, true, true),
        render(&r, Format::Json, true, true)
    );
}
#[test]
fn original_json_roundtrips() {
    let r = fixture();
    let out: Report = serde_json::from_str(&render(&r, Format::Json, false, true)).unwrap();
    assert_eq!(r.interpreter, out.interpreter);
    assert!(!out.redacted);
}
#[test]
fn markup_escapes_untrusted_values() {
    let r = fixture();
    for f in [Format::Html, Format::Markdown] {
        let s = render(&r, f, false, true);
        assert!(!s.contains("<tag attr"));
        assert!(s.contains("&lt;tag"));
    }
}
#[test]
fn summary_counts_cover_all_statuses() {
    let s = render(&fixture(), Format::Summary, true, true);
    assert!(s.contains("Warning 1"));
    assert!(s.contains("Skipped 0"));
    assert!(s.contains("Failure"));
}
