#![cfg(windows)]
use envlens_core::{discovery, model::Status, scan};

#[test]
fn windows_native_inventory_and_targeted_scan_return_real_observations() {
    let scope = discovery::discover(None);
    let candidate = scope
        .candidates
        .first()
        .map(|p| std::path::Path::new(&p.path));
    let mut stages = vec![];
    let report = scan(&scope, candidate, false, |_, stage| {
        stages.push(stage.to_owned())
    });
    assert!(stages.starts_with(&[
        "system".into(),
        "gpu".into(),
        "python".into(),
        "torch".into()
    ]));
    let os = report.checks.iter().find(|c| c.id == "system.os").unwrap();
    assert_eq!(
        os.status,
        Status::Ok,
        "Windows version query failed: {:?}",
        os.evidence
    );
    assert!(os
        .evidence
        .iter()
        .any(|f| f.key == "os" && f.value.contains("Windows")));
    assert_eq!(
        report.interpreter.as_deref(),
        candidate.and_then(|p| p.to_str())
    );
    for c in &report.checks {
        println!("{} {:?} | {}", c.id, c.status, c.summary.en);
    }
    let smoke = report
        .checks
        .iter()
        .find(|c| c.id == "torch.smoke")
        .unwrap();
    assert_eq!(smoke.status, Status::Skipped);
}
