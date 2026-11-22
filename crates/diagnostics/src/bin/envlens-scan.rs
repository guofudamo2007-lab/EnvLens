use envlens_core::{
    discovery::{discover, python_path},
    report::{render, Format},
    scan,
};
use std::{env, path::PathBuf, process::ExitCode};
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let (mut python, mut project) = (None, None);
    let (mut list, mut verify, mut private, mut english) = (false, false, false, false);
    let mut format = Format::Json;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("envlens-scan [--list] [--python FILE | --project DIR] [--verify-gpu] [--format json|md|html|summary] [--english] [--include-private]");
                return Ok(());
            }
            "--python" => {
                python = Some(PathBuf::from(
                    args.next().ok_or("--python requires a file")?,
                ))
            }
            "--project" => {
                project = Some(PathBuf::from(
                    args.next().ok_or("--project requires a directory")?,
                ))
            }
            "--format" => {
                format = match args.next().as_deref() {
                    Some("json") => Format::Json,
                    Some("md") => Format::Markdown,
                    Some("html") => Format::Html,
                    Some("summary") => Format::Summary,
                    _ => return Err("Unknown output format".into()),
                }
            }
            "--list" => list = true,
            "--verify-gpu" => verify = true,
            "--include-private" => private = true,
            "--english" => english = true,
            _ => return Err("Unknown option. See --help.".into()),
        }
    }
    if project.is_some() && python.is_some() {
        return Err("Choose --python or --project.".into());
    }
    let project = project
        .map(std::path::absolute)
        .transpose()
        .map_err(|_| "Invalid project path")?;
    if project.as_ref().is_some_and(|d| !d.is_dir()) {
        return Err("Project directory does not exist.".into());
    }
    let mut inv = discover(project.as_deref());
    if list {
        println!("{}", serde_json::to_string_pretty(&inv).unwrap());
        return Ok(());
    }
    let python = python
        .map(std::path::absolute)
        .transpose()
        .map_err(|_| "Invalid interpreter path")?;
    if let Some(exe) = &python {
        if !python_path(exe) {
            return Err("Choose an existing local python.exe.".into());
        }
        inv.add(exe, "manual");
    }
    let selected = python.or_else(|| inv.candidates.first().map(|e| PathBuf::from(&e.path)));
    let report = scan(&inv, selected.as_deref(), verify, |_, _| {});
    println!("{}", render(&report, format, !private, english));
    Ok(())
}
