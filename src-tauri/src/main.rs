#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod history;
use envlens_core::{
    discovery::{self, Inventory},
    model::{Report, DOCS},
    report::{self, Format},
};
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager, State};
#[derive(Default)]
struct Session {
    scope: Option<Inventory>,
    latest: Option<Report>,
}
#[derive(Default)]
struct Service {
    session: Mutex<Session>,
    startup_discovery: Mutex<Option<tauri::async_runtime::JoinHandle<Inventory>>>,
    busy: Arc<AtomicBool>,
}
struct Guard(Arc<AtomicBool>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
fn archive(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|p| p.join("history-v1"))
        .map_err(|_| "history_read".into())
}
fn session(s: &Service) -> Result<std::sync::MutexGuard<'_, Session>, String> {
    s.session.lock().map_err(|_| "internal".into())
}
#[tauri::command]
fn app_version() -> &'static str {
    envlens_core::VERSION
}
#[tauri::command]
async fn discover_targets(
    state: State<'_, Service>,
    at_startup: Option<bool>,
) -> Result<Inventory, String> {
    let started = if at_startup == Some(true) {
        state
            .startup_discovery
            .lock()
            .map_err(|_| "internal")?
            .take()
    } else {
        None
    };
    let job = started
        .unwrap_or_else(|| tauri::async_runtime::spawn_blocking(|| discovery::discover(None)));
    let inv = job.await.map_err(|_| "internal")?;
    session(&state)?.scope = Some(inv.clone());
    Ok(inv)
}
#[tauri::command]
async fn choose_target(
    kind: String,
    state: State<'_, Service>,
) -> Result<Option<Inventory>, String> {
    let inv = match kind.as_str() {
        "project" => {
            let Some(dir) = rfd::AsyncFileDialog::new().pick_folder().await else {
                return Ok(None);
            };
            let p = dir.path().to_owned();
            tauri::async_runtime::spawn_blocking(move || discovery::discover(Some(&p)))
                .await
                .map_err(|_| "internal")?
        }
        "python" => {
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("Python", &["exe"])
                .pick_file()
                .await
            else {
                return Ok(None);
            };
            if !discovery::python_path(file.path()) {
                return Err("invalid_python".into());
            }
            let mut inv = Inventory::default();
            inv.add(file.path(), "manual");
            inv
        }
        _ => return Err("invalid_target".into()),
    };
    session(&state)?.scope = Some(inv.clone());
    Ok(Some(inv))
}
#[derive(Clone, Serialize)]
struct Progress {
    step: u8,
    total: u8,
    stage: String,
}
#[derive(Serialize)]
struct ScanResult {
    report: Report,
    history_error: Option<String>,
}
#[tauri::command]
async fn run_scan(
    app: AppHandle,
    state: State<'_, Service>,
    target: Option<String>,
    verify_gpu: bool,
) -> Result<ScanResult, String> {
    if state
        .busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("busy".into());
    }
    let guard = Guard(state.busy.clone());
    let inv = session(&state)?.scope.clone().ok_or("scope_missing")?;
    let path = match target {
        Some(value) => Some(PathBuf::from(
            &inv.candidates
                .iter()
                .find(|c| c.path == value)
                .ok_or("invalid_target")?
                .path,
        )),
        None if inv.candidates.is_empty() => None,
        None => return Err("invalid_target".into()),
    };
    let sender = app.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        envlens_core::scan(&inv, path.as_deref(), verify_gpu, |step, stage| {
            let _ = sender.emit_to(
                "main",
                "scan-progress",
                Progress {
                    step,
                    total: 5,
                    stage: stage.into(),
                },
            );
        })
    })
    .await
    .map_err(|_| "internal")?;
    session(&state)?.latest = Some(report.clone());
    let dir = archive(&app);
    let (report, history_error) = tauri::async_runtime::spawn_blocking(move || {
        let error = dir.and_then(|path| history::save(&path, &report)).err();
        (report, error)
    })
    .await
    .map_err(|_| "internal")?;
    drop(guard);
    Ok(ScanResult {
        report,
        history_error,
    })
}
#[tauri::command]
async fn list_history(app: AppHandle) -> Result<history::History, String> {
    let dir = archive(&app)?;
    tauri::async_runtime::spawn_blocking(move || history::load(&dir))
        .await
        .map_err(|_| "internal")?
}
async fn lookup(app: &AppHandle, state: &Service, id: &str) -> Result<Report, String> {
    if let Some(r) = &session(state)?.latest {
        if r.id == id {
            return Ok(r.clone());
        }
    }
    let dir = archive(app)?;
    let wanted = id.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        history::load(&dir)?
            .reports
            .into_iter()
            .find(|r| r.id == wanted)
            .ok_or_else(|| "report_missing".into())
    })
    .await
    .map_err(|_| "internal")?
}
#[tauri::command]
async fn preview_report(
    app: AppHandle,
    state: State<'_, Service>,
    id: String,
    format: Format,
    redact: bool,
    english: bool,
) -> Result<String, String> {
    let report = lookup(&app, &state, &id).await?;
    Ok(report::render(&report, format, redact, english))
}
#[tauri::command]
async fn save_report(
    app: AppHandle,
    state: State<'_, Service>,
    id: String,
    format: Format,
    redact: bool,
    english: bool,
) -> Result<bool, String> {
    let r = lookup(&app, &state, &id).await?;
    let extension = format.extension();
    let suggested = format!(
        "EnvLens-{}-{}.{}",
        r.started_at.get(..10).unwrap_or("scan"),
        r.id.get(..8).unwrap_or("local"),
        extension
    );
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_file_name(suggested)
        .add_filter(extension, &[extension])
        .save_file()
        .await
    else {
        return Ok(false);
    };
    let dest = file.path();
    let dir = dest.parent().unwrap_or(Path::new("."));
    if !dest
        .extension()
        .is_some_and(|x| x.to_string_lossy().eq_ignore_ascii_case(extension))
    {
        return Err("wrong_extension".into());
    }
    let data = report::render(&r, format, redact, english);
    let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(|_| "export_write")?;
    tmp.write_all(data.as_bytes())
        .and_then(|_| tmp.as_file().sync_all())
        .map_err(|_| "export_write")?;
    tmp.persist(dest).map_err(|_| "export_write")?;
    Ok(true)
}
#[tauri::command]
fn open_doc(url: String) -> Result<(), String> {
    if !DOCS.contains(&url.as_str()) {
        return Err("invalid_document".into());
    }
    open::that(url).map_err(|_| "open_document".into())
}
fn main() {
    tauri::Builder::default()
        .manage(Service::default())
        .setup(|app| {
            // Start the first inventory while Windows creates the WebView. Explicit refreshes always probe again.
            let service = app.state::<Service>();
            *service
                .startup_discovery
                .lock()
                .map_err(|_| std::io::Error::other("startup discovery lock"))? =
                Some(tauri::async_runtime::spawn_blocking(|| {
                    discovery::discover(None)
                }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            discover_targets,
            choose_target,
            run_scan,
            list_history,
            preview_report,
            save_report,
            open_doc
        ])
        .run(tauri::generate_context!())
        .expect("EnvLens desktop runtime failed");
}
