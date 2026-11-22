pub mod discovery;
pub mod gpu;
pub mod model;
pub mod ollama;
pub mod python;
pub mod report;
pub mod runner;
pub mod system;
pub mod torch;

use discovery::Inventory;
use model::Report;
use std::{path::Path, time::Instant};
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn scan(
    inv: &Inventory,
    path: Option<&Path>,
    verify_gpu: bool,
    mut progress: impl FnMut(u8, &str),
) -> Report {
    let start = Instant::now();
    let started_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    progress(0, "system");
    let host = system::collect();
    let mut checks = system::checks(&host);
    progress(1, "gpu");
    let (hardware, adapter_checks) = gpu::collect(&host);
    checks.extend(adapter_checks);
    progress(2, "python");
    checks.push(python::discovery_check(inv));
    let (info, py_checks) = python::collect(path);
    checks.extend(py_checks);
    progress(3, "torch");
    checks.extend(torch::collect(path, info.as_ref(), &hardware, verify_gpu));
    progress(4, "ollama");
    checks.extend(ollama::collect(&host));
    progress(5, "complete");
    let mut report = Report {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        started_at,
        duration_ms: start.elapsed().as_millis() as u64,
        interpreter: path.map(|p| p.to_string_lossy().into()),
        project: inv.project.clone(),
        redacted: false,
        checks,
    };
    report::scrub_credentials(&mut report);
    report
}
