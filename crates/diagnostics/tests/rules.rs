use envlens_core::{
    discovery::launcher_paths,
    gpu::{parse_smi, Card, Gpu},
    model::{Confidence, Status},
    runner::Fault,
    torch::{evaluate, Torch},
};

fn hardware() -> Gpu {
    Gpu {
        cards: vec![Card {
            name: "fixture GPU".into(),
            driver: "550.54".into(),
            memory_mib: Some(8192),
        }],
        nvidia: Some(true),
        smi: Ok(()),
    }
}
fn runtime() -> Torch {
    Torch {
        installed: true,
        version: Some("2.5".into()),
        cuda: Some("12.4".into()),
        ..Default::default()
    }
}
fn diagnosis(t: &Torch, g: &Gpu) -> Vec<envlens_core::model::Check> {
    evaluate(t, g, None, r"C:\Tools\Python\python.exe", false)
}
#[test]
fn cpu_build_with_nvidia_is_confirmed() {
    let t = Torch {
        cuda: None,
        ..runtime()
    };
    let c = diagnosis(&t, &hardware());
    assert_eq!(c[1].status, Status::Warning);
    assert_eq!(
        c[1].finding.as_ref().unwrap().confidence,
        Confidence::Confirmed
    );
}
#[test]
fn a_missing_optional_runtime_is_skipped() {
    let c = diagnosis(&Torch::default(), &hardware());
    assert_eq!(c[0].status, Status::Skipped);
    assert_eq!(c[1].status, Status::Skipped);
}
#[test]
fn metadata_and_failed_import_differ_from_absence() {
    let t = Torch {
        error: Some("OSError".into()),
        ..Default::default()
    };
    let c = evaluate(&t, &hardware(), Some("2.4"), "python.exe", false);
    assert_eq!(c[0].status, Status::Error);
}
#[test]
fn positive_enumeration_does_not_claim_computation() {
    let t = Torch {
        available: Some(true),
        count: Some(1),
        devices: vec!["GPU".into()],
        ..runtime()
    };
    let c = diagnosis(&t, &hardware());
    assert_eq!(c[1].status, Status::Ok);
    assert_eq!(c[2].status, Status::Skipped);
}
#[test]
fn failure_to_compute_is_separate() {
    let t = Torch {
        available: Some(true),
        count: Some(1),
        smoke: Some(false),
        ..runtime()
    };
    let c = evaluate(&t, &hardware(), None, "python.exe", true);
    assert_eq!(c[1].status, Status::Ok);
    assert_eq!(c[2].status, Status::Error);
}
#[test]
fn driver_floor_is_only_a_likely_cause() {
    let mut g = hardware();
    g.cards[0].driver = "512.96".into();
    let c = diagnosis(&runtime(), &g);
    assert_eq!(
        c[1].finding.as_ref().unwrap().confidence,
        Confidence::Likely
    );
    assert!(c[1].summary.en.contains("incompatible"));
}
#[test]
fn future_cuda_versions_are_not_guessed() {
    let t = Torch {
        cuda: Some("90.1".into()),
        ..runtime()
    };
    assert_eq!(diagnosis(&t, &hardware())[1].status, Status::Unknown);
}
#[test]
fn visibility_has_its_own_evidence() {
    let t = Torch {
        mask: true,
        ..runtime()
    };
    let c = diagnosis(&t, &hardware());
    assert!(c[1].evidence.iter().any(|e| e.key == "visibility"));
    assert_eq!(
        c[1].finding.as_ref().unwrap().confidence,
        Confidence::Confirmed
    );
}
#[test]
fn failed_init_does_not_pin_blame_on_driver() {
    let t = Torch {
        init_error: Some("RuntimeError".into()),
        ..runtime()
    };
    let c = diagnosis(&t, &hardware());
    assert_eq!(c[1].status, Status::Error);
    assert_eq!(
        c[1].finding.as_ref().unwrap().confidence,
        Confidence::Undetermined
    );
}
#[test]
fn cuda_error_reporting_missing_driver_is_preserved() {
    let t = Torch {
        init_error: Some("driver_not_found".into()),
        ..runtime()
    };
    let c = diagnosis(&t, &hardware());
    assert!(c[1].summary.en.contains("driver"));
    assert!(c[1].evidence.iter().any(|e| e.value == "driver_not_found"));
}
#[test]
fn inaccessible_hardware_is_not_a_missing_driver() {
    let g = Gpu {
        cards: vec![],
        nvidia: None,
        smi: Err(Fault::Denied),
    };
    let c = diagnosis(&runtime(), &g);
    assert_eq!(c[1].status, Status::Unknown);
    assert!(c[1].evidence.iter().any(|e| e.value == "permission_denied"));
}
#[test]
fn parses_multiple_adapters_and_unavailable_memory() {
    let cards = parse_smi(b"RTX A, 551.2, 4096\r\nRTX B, 551.2, [N/A]\r\n").unwrap();
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[1].memory_mib, None);
    assert!(parse_smi(b"error").is_err());
}
#[test]
fn launcher_accepts_legacy_and_modern_layouts() {
    let data = include_bytes!("fixtures/launcher.txt");
    let p = launcher_paths(data);
    assert_eq!(p.len(), 3);
    assert!(p[0].0);
    assert!(p
        .iter()
        .any(|(_, p)| p.to_string_lossy().contains("AI Tools")));
}
