use crate::model::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::LazyLock;
static SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\b(?:gh[pousr]_|github_pat_|sk-|hf_)[a-z0-9_-]{12,}|\bbearer\s+[a-z0-9_.~+/-]+=*|\b(?:api[_-]?key|password|access[_-]?token)\s*[:=]\s*[^\s&;,]+|https?://[^/\s]+:[^@\s]+@)").expect("static credential pattern")
});
pub fn scrub_credentials(r: &mut Report) {
    fn clean(v: &mut String) {
        if SECRET.is_match(v) {
            *v = SECRET.replace_all(v, "[REDACTED]").into_owned();
        }
    }
    for v in [&mut r.project, &mut r.interpreter].into_iter().flatten() {
        clean(v)
    }
    for check in &mut r.checks {
        for e in &mut check.evidence {
            clean(&mut e.value)
        }
        if let Some(f) = &mut check.finding {
            for command in &mut f.commands {
                clean(command)
            }
        }
    }
}
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Json,
    Markdown,
    Html,
    Summary,
}
impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Html => "html",
            Self::Markdown => "md",
            Self::Summary => "txt",
        }
    }
}

pub fn redact(report: &Report) -> Report {
    let mut r = report.clone();
    r.redacted = true;
    if r.interpreter.is_some() {
        r.interpreter = Some("<python.exe>".into());
    }
    if r.project.is_some() {
        r.project = Some("<project>".into());
    }
    for c in &mut r.checks {
        for e in &mut c.evidence {
            if e.private {
                e.value = match e.key.as_str() {
                    "model" => "<model>",
                    "requested" | "sys.executable" => "<python.exe>",
                    _ => "<private>",
                }
                .into();
            }
        }
        if let Some(f) = &mut c.finding {
            for cmd in &mut f.commands {
                if let Some(path) = &report.interpreter {
                    *cmd =
                        cmd.replace(&format!("'{}'", path.replace('\'', "''")), "'<python.exe>'");
                }
            }
        }
    }
    r
}
pub fn status(s: Status, en: bool) -> &'static str {
    match (s, en) {
        (Status::Ok, true) => "Passed",
        (Status::Warning, true) => "Warning",
        (Status::Error, true) => "Error",
        (Status::Unknown, true) => "Unknown",
        (Status::Skipped, true) => "Skipped",
        (Status::Ok, false) => "正常",
        (Status::Warning, false) => "警告",
        (Status::Error, false) => "异常",
        (Status::Unknown, false) => "无法确定",
        (Status::Skipped, false) => "未检查",
    }
}
fn confidence(v: Confidence, en: bool) -> &'static str {
    match (v, en) {
        (Confidence::Confirmed, true) => "Confirmed",
        (Confidence::Confirmed, false) => "已确认",
        (Confidence::Likely, true) => "Likely",
        (Confidence::Likely, false) => "可能",
        (Confidence::Undetermined, true) => "Undetermined",
        (Confidence::Undetermined, false) => "待确认",
    }
}
fn one_line(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn md(s: &str) -> String {
    let mut out = String::new();
    for c in one_line(s).chars() {
        if "\\\x60*_[]()#|!~".contains(c) {
            out.push('\\');
        }
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}
fn tr<'a>(en: bool, zh: &'a str, english: &'a str) -> &'a str {
    if en {
        english
    } else {
        zh
    }
}
fn counts(r: &Report, en: bool) -> String {
    [
        Status::Ok,
        Status::Warning,
        Status::Error,
        Status::Unknown,
        Status::Skipped,
    ]
    .map(|s| {
        format!(
            "{} {}",
            status(s, en),
            r.checks.iter().filter(|c| c.status == s).count()
        )
    })
    .join(" · ")
}
fn level(v: &str, en: bool) -> &str {
    if en {
        return v;
    }
    match v {
        "high" => "高",
        "medium" => "中",
        _ => "信息",
    }
}
pub fn render(r: &Report, format: Format, hide: bool, en: bool) -> String {
    let masked;
    let r = if hide {
        masked = redact(r);
        &masked
    } else {
        r
    };
    if format == Format::Json {
        return serde_json::to_string_pretty(r).expect("serializable diagnostic report");
    }
    let title = tr(en, "EnvLens 环境诊断报告", "EnvLens environment report");
    let count = counts(r, en);
    let interpreter = r.interpreter.as_deref().unwrap_or("—");
    let meta = format!(
        "{} · {} ms · {} · {}",
        r.started_at,
        r.duration_ms,
        interpreter,
        if r.redacted {
            tr(en, "已脱敏", "Redacted")
        } else {
            tr(en, "原始路径", "Original paths")
        }
    );
    if format == Format::Summary {
        let mut s = format!("{title}\n{meta}\n{count}\n");
        for c in r.checks.iter().filter(|c| c.finding.is_some()) {
            let _ = writeln!(
                s,
                "• [{}] {} — {}",
                status(c.status, en),
                c.title.get(en),
                c.summary.get(en)
            );
        }
        return s;
    }
    let is_html = format == Format::Html;
    let mut out = if is_html {
        format!("<!doctype html><html lang=\"{}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'\"><title>{title}</title><style>html{{color-scheme:light dark}}body{{max-width:850px;margin:3rem auto;padding:0 1.3rem;font:16px/1.65 system-ui;overflow-wrap:anywhere}}section{{border-top:1px solid #82928d;padding:1rem 0}}h2{{font-size:1.2rem}}code,pre{{white-space:pre-wrap}}a{{color:#398b79}}small{{color:#808b8a}}</style><h1>{title}</h1><p>{}</p><strong>{}</strong>",if en{"en"}else{"zh-CN"},html(&meta),html(&count))
    } else {
        format!("# {title}\n\n{}\n\n{count}\n", md(&meta))
    };
    if let Some(project) = &r.project {
        line(&mut out, is_html, tr(en, "项目", "Project"), project);
    }
    for c in &r.checks {
        let heading = format!("{} · {}", status(c.status, en), c.title.get(en));
        if is_html {
            let _ = write!(
                out,
                "<section><h2>{}</h2><p>{}</p>",
                html(&heading),
                html(c.summary.get(en))
            );
        } else {
            let _ = writeln!(out, "\n## {}\n\n{}\n", md(&heading), md(c.summary.get(en)));
        }
        for f in &c.evidence {
            line(&mut out, is_html, &f.key, &f.value);
        }
        if let Some(f) = &c.finding {
            line(
                &mut out,
                is_html,
                tr(en, "严重度", "Severity"),
                level(&f.severity, en),
            );
            line(
                &mut out,
                is_html,
                tr(en, "置信程度", "Confidence"),
                confidence(f.confidence, en),
            );
            line(
                &mut out,
                is_html,
                tr(en, "影响", "Impact"),
                f.impact.get(en),
            );
            line(&mut out, is_html, tr(en, "原因", "Cause"), f.cause.get(en));
            for (i, s) in f.steps.iter().enumerate() {
                line(
                    &mut out,
                    is_html,
                    &format!("{} {}", tr(en, "步骤", "Step"), i + 1),
                    s.get(en),
                );
            }
            for cmd in &f.commands {
                if is_html {
                    let _ = write!(out, "<pre><code>{}</code></pre>", html(cmd));
                } else {
                    let _ = writeln!(out, "\n    {}\n", one_line(cmd));
                }
            }
            for url in &f.docs {
                if DOCS.contains(&url.as_str()) {
                    if is_html {
                        let _ = write!(out, "<p><a href=\"{}\">{}</a></p>", html(url), html(url));
                    } else {
                        let _ = writeln!(out, "\n<{}>\n", url);
                    }
                }
            }
        }
        if is_html {
            out.push_str("</section>");
        }
    }
    if is_html {
        out.push_str("</html>");
    }
    out
}
fn line(out: &mut String, is_html: bool, key: &str, value: &str) {
    if is_html {
        let _ = write!(out, "<div><b>{}</b> &nbsp;{}</div>", html(key), html(value));
    } else {
        let _ = writeln!(out, "- **{}** · {}", md(key), md(value));
    }
}
