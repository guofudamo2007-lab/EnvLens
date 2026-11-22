use envlens_core::{model::Report, report::redact};
#[cfg(test)]
mod tests;
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
const RETAIN: usize = 20;
#[derive(Default, Serialize)]
pub struct History {
    pub reports: Vec<Report>,
    pub unreadable: usize,
}
fn entries(dir: &Path) -> Result<(Vec<(PathBuf, Report)>, usize), String> {
    let iter = match fs::read_dir(dir) {
        Ok(i) => i,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((vec![], 0)),
        Err(_) => return Err("history_read".into()),
    };
    let (mut rows, mut unreadable) = (vec![], 0);
    for entry in iter {
        let Ok(e) = entry else {
            unreadable += 1;
            continue;
        };
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let read = (|| {
            let f = fs::File::open(&path).ok()?;
            if f.metadata().ok()?.len() > 2_000_000 {
                return None;
            }
            let mut b = vec![];
            f.take(2_000_001).read_to_end(&mut b).ok()?;
            let r: Report = serde_json::from_slice(&b).ok()?;
            if r.schema_version != 1 || !r.redacted || path.file_stem()?.to_str()? != r.id {
                return None;
            }
            Some(r)
        })();
        if let Some(r) = read {
            rows.push((path, r))
        } else {
            unreadable += 1
        }
    }
    rows.sort_by(|a, b| {
        b.1.started_at
            .cmp(&a.1.started_at)
            .then_with(|| b.1.id.cmp(&a.1.id))
    });
    Ok((rows, unreadable))
}
pub fn load(dir: &Path) -> Result<History, String> {
    let (rows, unreadable) = entries(dir)?;
    Ok(History {
        reports: rows.into_iter().take(RETAIN).map(|(_, r)| r).collect(),
        unreadable,
    })
}
pub fn save(dir: &Path, report: &Report) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|_| "history_write")?;
    let bytes = serde_json::to_vec(&redact(report)).map_err(|_| "history_write")?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|_| "history_write")?;
    temp.write_all(&bytes)
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|_| "history_write")?;
    temp.persist_noclobber(dir.join(format!("{}.json", report.id)))
        .map_err(|_| "history_write")?;
    for (path, _) in entries(dir)?.0.into_iter().skip(RETAIN) {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("history_prune".into()),
        }
    }
    Ok(())
}
