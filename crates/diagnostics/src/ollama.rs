use crate::{
    model::*,
    runner,
    system::{failed, Host},
};
use serde::Deserialize;
use std::{env, io::ErrorKind, path::PathBuf, time::Duration};

#[derive(Debug, PartialEq, Eq)]
enum ApiError {
    Refused,
    Denied,
    Http(u16),
    Timeout,
    Invalid,
    Transport,
}
impl ApiError {
    fn code(&self) -> String {
        match self {
            Self::Refused => "connection_refused".into(),
            Self::Denied => "permission_denied".into(),
            Self::Http(n) => format!("HTTP {n}"),
            Self::Timeout => "timeout".into(),
            Self::Invalid => "invalid_response".into(),
            Self::Transport => "transport_error".into(),
        }
    }
}
fn map_error(e: ureq::Error) -> ApiError {
    match e {
        ureq::Error::Timeout(_) => ApiError::Timeout,
        ureq::Error::Io(e) => match e.kind() {
            ErrorKind::ConnectionRefused => ApiError::Refused,
            ErrorKind::PermissionDenied => ApiError::Denied,
            ErrorKind::TimedOut => ApiError::Timeout,
            _ => ApiError::Transport,
        },
        _ => ApiError::Transport,
    }
}
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_millis(2200)))
        .build()
        .into()
}
fn get<T: serde::de::DeserializeOwned>(
    client: &ureq::Agent,
    port: u16,
    route: &str,
) -> Result<T, ApiError> {
    let mut resp = client
        .get(format!("http://127.0.0.1:{port}{route}"))
        .call()
        .map_err(map_error)?;
    if resp.status().as_u16() != 200 {
        return Err(ApiError::Http(resp.status().as_u16()));
    }
    let body = resp
        .body_mut()
        .with_config()
        .limit(256 * 1024)
        .read_to_vec()
        .map_err(|_| ApiError::Invalid)?;
    serde_json::from_slice(&body).map_err(|_| ApiError::Invalid)
}
#[derive(Deserialize)]
struct Version {
    version: String,
}
#[derive(Deserialize)]
struct Model {
    name: String,
}
#[derive(Deserialize)]
struct Tags {
    models: Vec<Model>,
}
fn version(s: &str) -> Option<&str> {
    s.split_whitespace()
        .map(|s| s.trim_matches(|c: char| c == ',' || c == 'v'))
        .find(|v| {
            let mut parts = v.split('.');
            v.len() < 64
                && parts.next().is_some_and(|p| p.parse::<u32>().is_ok())
                && parts.next().is_some_and(|p| p.parse::<u32>().is_ok())
                && v.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".+-".contains(&b))
        })
}
fn cli_version(output: &str) -> Option<&str> {
    // When they differ, the first line identifies the server and the warning identifies the CLI.
    for prefix in ["Warning: client version is ", "ollama version is "] {
        if let Some(raw) = output.lines().find_map(|s| s.trim().strip_prefix(prefix)) {
            return version(raw).filter(|v| *v == raw);
        }
    }
    None
}
pub fn collect(host: &Host) -> Vec<Check> {
    let install = ("Ollama 程序", "Ollama installation");
    let process = ("Ollama 进程", "Ollama process");
    let path = env::var_os("LOCALAPPDATA")
        .map(|d| PathBuf::from(d).join(r"Programs\Ollama\ollama.exe"))
        .into_iter()
        .chain(runner::path_exes("ollama.exe"))
        .find(|p| p.is_file());
    let cli = match &path {
        Some(exe) => match runner::success(exe, &["--version"], 6) {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out);
                if let Some(v) = cli_version(&text) {
                    Check::new(
                        "ollama.install",
                        Group::Ollama,
                        install,
                        Status::Ok,
                        ("已读取本地 CLI 版本。", "Read the local CLI version."),
                    )
                    .private("executable", exe.display())
                    .fact("cli_version", v)
                } else {
                    failed(
                        "ollama.install",
                        Group::Ollama,
                        install,
                        "ollama --version",
                        runner::Fault::Invalid,
                    )
                    .private("executable", exe.display())
                }
            }
            Err(e) => failed(
                "ollama.install",
                Group::Ollama,
                install,
                "ollama --version",
                e,
            )
            .private("executable", exe.display()),
        },
        None => Check::new(
            "ollama.install",
            Group::Ollama,
            install,
            Status::Skipped,
            (
                "已知安装目录和 PATH 中未发现 Ollama。",
                "Ollama was not found in its standard location or PATH.",
            ),
        )
        .fact("lookup", "standard location / PATH"),
    };
    let running = match host.ollama {
        Ok(true) => Check::new(
            "ollama.process",
            Group::Ollama,
            process,
            Status::Ok,
            (
                "检测到本机 Ollama 进程。",
                "An Ollama process was observed.",
            ),
        )
        .fact("running", true),
        Ok(false) => Check::new(
            "ollama.process",
            Group::Ollama,
            process,
            Status::Skipped,
            (
                "采样时未观察到 Ollama 进程。",
                "No Ollama process was observed at sampling time.",
            ),
        )
        .fact("running", false),
        Err(e) => failed(
            "ollama.process",
            Group::Ollama,
            process,
            "Process32FirstW",
            e,
        ),
    };
    let mut checks = vec![cli, running];
    checks.extend(api_checks(11434, path.is_some() || host.ollama == Ok(true)));
    checks.push(Check::new(
        "ollama.inference",
        Group::Ollama,
        ("模型推理", "Model inference"),
        Status::Skipped,
        (
            "本次体检未加载模型或执行推理。",
            "The scan did not load a model or run inference.",
        ),
    ));
    checks
}
fn api_checks(port: u16, expected: bool) -> Vec<Check> {
    let client = agent();
    let api = ("本地服务端点", "Local service endpoint");
    let models = ("本地模型列表", "Local model list");
    let reply = get::<Version>(&client, port, "/api/version").and_then(|v| {
        if version(&v.version) == Some(v.version.as_str()) {
            Ok(v)
        } else {
            Err(ApiError::Invalid)
        }
    });
    let offline = matches!(reply, Err(ApiError::Refused));
    let service=match reply {
        Ok(v)=>Check::new("ollama.api",Group::Ollama,api,Status::Ok,("默认回环端点返回了服务版本。","The default loopback endpoint returned a service version.")).fact("server_version",v.version),
        Err(ApiError::Refused) if !expected=>Check::new("ollama.api",Group::Ollama,api,Status::Skipped,("默认端口拒绝了连接，未观察到本机 Ollama。","The default port refused the connection and no local Ollama was observed.")).fact("response","connection_refused"),
        Err(e)=>{let status=if matches!(e,ApiError::Refused)&&expected{Status::Warning}else{Status::Unknown};
            Check::new("ollama.api",Group::Ollama,api,status,("默认端点未返回可验证的 Ollama 版本。","The default endpoint did not return a verifiable Ollama version.")).fact("response",e.code())
            .explain(Confidence::Undetermined,("依赖默认端点的客户端可能无法连接。","Clients using the default endpoint may be unable to connect."),("请核对服务是否已启动、是否使用了自定义端口，以及本地访问权限。","The service may be stopped, listening elsewhere, or inaccessible to this user."),
                &[("打开 Ollama 并在同一用户会话核对默认端点，然后复检。","Open Ollama and verify the default endpoint in the same user session, then rescan.")],&[OLLAMA_DOC])
            .command("Invoke-RestMethod 'http://127.0.0.1:11434/api/version'".into())}
    }.fact("endpoint",format!("127.0.0.1:{port}/api/version"));
    let tags = if offline {
        Check::new(
            "ollama.models",
            Group::Ollama,
            models,
            Status::Skipped,
            (
                "默认端口拒绝连接，未请求模型列表。",
                "The default port refused the connection, so the model list was skipped.",
            ),
        )
    } else {
        match get::<Tags>(&client, port, "/api/tags") {
            Ok(tags) => {
                let mut c = Check::new(
                    "ollama.models",
                    Group::Ollama,
                    models,
                    Status::Ok,
                    ("已读取本地模型清单。", "Read the local model list."),
                )
                .fact("model_count", tags.models.len());
                for m in tags.models.into_iter().take(40) {
                    c = c.private(
                        "model",
                        m.name
                            .chars()
                            .filter(|c| !c.is_control())
                            .take(160)
                            .collect::<String>(),
                    );
                }
                c
            }
            Err(e) => Check::new(
                "ollama.models",
                Group::Ollama,
                models,
                Status::Unknown,
                (
                    "模型清单未通过验证。",
                    "The model list could not be verified.",
                ),
            )
            .fact("response", e.code())
            .explain(
                Confidence::Undetermined,
                (
                    "当前模型数量未知。",
                    "The number of local models is unknown.",
                ),
                (
                    "模型端点返回失败或格式与预期不符。",
                    "The model endpoint failed or returned unexpected data.",
                ),
                &[(
                    "运行 ollama list，核对本地清单后复检。",
                    "Run ollama list to verify the local inventory and rescan.",
                )],
                &[TAGS_DOC],
            )
            .command("ollama list".into()),
        }
    }
    .fact("endpoint", format!("127.0.0.1:{port}/api/tags"));
    vec![service, tags]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_client_version_from_server_version() {
        assert_eq!(
            cli_version("ollama version is 0.8.0\nWarning: client version is 0.9.1\n"),
            Some("0.9.1")
        );
        assert_eq!(cli_version("could not connect to 127.0.0.1"), None);
    }
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };
    fn serve(responses: Vec<&'static str>) -> (u16, thread::JoinHandle<()>) {
        let server = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = server.local_addr().unwrap().port();
        let task = thread::spawn(move || {
            for content in responses {
                let (mut stream, _) = server.accept().unwrap();
                let mut bytes = [0; 2048];
                let _ = stream.read(&mut bytes).unwrap();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",content.len(),content).unwrap();
            }
        });
        (port, task)
    }
    #[test]
    fn service_and_tags_are_independent() {
        let (port, task) = serve(vec![
            r#"{"version":"0.9.1"}"#,
            r#"{"models":[{"name":"private/model"}]}"#,
        ]);
        let c = api_checks(port, true);
        task.join().unwrap();
        assert_eq!(c[0].status, Status::Ok);
        assert!(c[1].evidence.iter().any(|e| e.key == "model" && e.private));
        let (port, task) = serve(vec![r#"{"version":"0.9.1"}"#, "{}"]);
        let c = api_checks(port, true);
        task.join().unwrap();
        assert_eq!(c[1].status, Status::Unknown);
    }
}
