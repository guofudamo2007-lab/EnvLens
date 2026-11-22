use crate::{
    discovery::{same, Inventory},
    model::*,
    runner::{self, Fault},
    system::failed,
};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path, time::Duration};
#[derive(Debug, Deserialize)]
pub struct Python {
    pub executable: String,
    pub version: String,
    pub prefix: String,
    pub base_prefix: String,
    pub kind: String,
    pub base_exists: bool,
    pub packages: BTreeMap<String, String>,
}
pub fn probe(exe: &Path, script: &str, arg: &str, seconds: u64) -> Result<runner::Output, Fault> {
    let code = format!("{}\n{}", include_str!("../probes/bootstrap.py"), script);
    runner::run(
        exe,
        &["-I", "-S", "-B", "-X", "utf8", "-c", &code, arg],
        Duration::from_secs(seconds),
    )
}
pub fn json<T: serde::de::DeserializeOwned>(output: runner::Output) -> Result<T, Fault> {
    if output.code != Some(0) {
        return Err(Fault::Failed);
    }
    serde_json::from_slice(&output.stdout).map_err(|_| Fault::Invalid)
}
pub fn discovery_check(inv: &Inventory) -> Check {
    let title = ("解释器发现", "Interpreter discovery");
    let mut c = if inv.project.is_some() && inv.candidates.is_empty() {
        Check::new(
            "python.discovery",
            Group::Python,
            title,
            Status::Warning,
            (
                "所选项目中未找到约定位置的解释器。",
                "No interpreter found at the selected project’s conventional locations.",
            ),
        )
        .explain(
            Confidence::Confirmed,
            (
                "此项目的 Python 与 PyTorch 检查已跳过。",
                "Python and PyTorch checks for this project were skipped.",
            ),
            (
                "已检查项目根目录以及 .venv、venv、env 和便携 Python 目录。",
                "Checked the project root, .venv, venv, env and portable Python locations.",
            ),
            &[(
                "在 Python 环境页直接选择项目实际使用的 python.exe，再复检。",
                "Select the python.exe used by this project on the Python page and rescan.",
            )],
            &[VENV_DOC],
        )
    } else if inv.candidates.is_empty() {
        Check::new(
            "python.discovery",
            Group::Python,
            title,
            Status::Unknown,
            (
                "已知位置中未发现可检查的 Python。",
                "No inspectable Python was found in the known locations.",
            ),
        )
        .explain(
            Confidence::Undetermined,
            (
                "依赖与 CUDA 运行时尚未检查。",
                "Dependencies and the CUDA runtime remain untested.",
            ),
            (
                "解释器可能尚未安装，也可能位于自定义目录。",
                "Python may be absent or installed in a custom location.",
            ),
            &[(
                "从 Python 环境页选择实际使用的 python.exe。",
                "Choose the python.exe you use on the Python page.",
            )],
            &[PY_DOC],
        )
    } else if !inv.notices.is_empty() {
        Check::new(
            "python.discovery",
            Group::Python,
            title,
            Status::Warning,
            (
                "找到了可用候选项，部分发现来源未能验证。",
                "Candidates were found, but some discovery sources could not be verified.",
            ),
        )
        .explain(
            Confidence::Confirmed,
            (
                "候选解释器列表可能不完整。",
                "The candidate list may be incomplete.",
            ),
            (
                "发现来源返回了下列状态。",
                "A discovery source returned the recorded status.",
            ),
            &[(
                "核对当前启动环境或手动选择解释器。",
                "Check the launching environment or choose the interpreter manually.",
            )],
            &[PY_DOC],
        )
    } else {
        Check::new(
            "python.discovery",
            Group::Python,
            title,
            Status::Ok,
            (
                "已枚举当前范围内的解释器。",
                "Enumerated interpreters within the current scope.",
            ),
        )
    };
    c = c.fact("candidates", inv.candidates.len());
    if let Some(p) = &inv.launcher {
        c = c.private("launcher", p);
    }
    for n in &inv.notices {
        c = c.fact("discovery_note", n);
    }
    c
}
pub fn skipped(id: &str, group: Group, title: Pair) -> Check {
    Check::new(
        id,
        group,
        title,
        Status::Skipped,
        (
            "缺少已验证的上游环境。",
            "A verified prerequisite is unavailable.",
        ),
    )
}
pub fn collect(path: Option<&Path>) -> (Option<Python>, Vec<Check>) {
    let interpreter = ("当前解释器", "Selected interpreter");
    let env_title = ("虚拟环境", "Environment isolation");
    let pip = ("pip 可用性", "pip availability");
    let deps = ("依赖一致性", "Dependency consistency");
    let probe_result = path.map(|exe| {
        probe(exe, include_str!("../probes/python.py"), "", 10).and_then(json::<Python>)
    });
    let (exe, info) = match (path, probe_result) {
        (Some(exe), Some(Ok(info))) => (exe, info),
        (_, result) => {
            let mut c = match result {
                Some(Err(e)) => failed(
                    "python.runtime",
                    Group::Python,
                    interpreter,
                    "python -I -S",
                    e,
                ),
                _ => skipped("python.runtime", Group::Python, interpreter),
            };
            if let Some(p) = path {
                c = c.private("requested", p.display());
            }
            return (
                None,
                vec![
                    c,
                    skipped("python.env", Group::Python, env_title),
                    skipped("python.pip", Group::Python, pip),
                    skipped("python.deps", Group::Python, deps),
                ],
            );
        }
    };
    let requested = exe.to_string_lossy();
    let runtime = if same(&requested, &info.executable) {
        Check::new(
            "python.runtime",
            Group::Python,
            interpreter,
            Status::Ok,
            (
                "选定解释器已启动并返回自身路径。",
                "The selected interpreter started and reported its executable.",
            ),
        )
    } else {
        Check::new(
            "python.runtime",
            Group::Python,
            interpreter,
            Status::Warning,
            (
                "启动目标与 Python 自报路径不同。",
                "The requested and reported interpreter paths differ.",
            ),
        )
        .explain(
            Confidence::Confirmed,
            (
                "依赖来自实际启动的解释器。",
                "Dependencies belong to the interpreter that actually started.",
            ),
            (
                "启动器、链接或嵌入式环境可能重定向解释器。",
                "A launcher, link or embedded environment may redirect the interpreter.",
            ),
            &[(
                "对照项目启动命令，重新选择对应的 python.exe。",
                "Compare with the project launch command and select the matching python.exe.",
            )],
            &[PY_DOC],
        )
    }
    .private("requested", &requested)
    .private("sys.executable", &info.executable)
    .fact("python", &info.version);
    let environment = if info.base_exists {
        Check::new(
            "python.env",
            Group::Python,
            env_title,
            Status::Ok,
            (
                "已识别环境类型与基础解释器目录。",
                "Identified the environment type and the base interpreter directory.",
            ),
        )
    } else {
        Check::new(
            "python.env",
            Group::Python,
            env_title,
            Status::Warning,
            (
                "基础解释器目录不可见。",
                "The base interpreter directory is inaccessible.",
            ),
        )
        .explain(
            Confidence::Likely,
            (
                "环境可能无法读取基础安装的组件。",
                "The environment may have lost access to its base installation.",
            ),
            (
                "环境可能已移动，或基础安装位置已更改。",
                "The environment may have moved or the base installation may have changed.",
            ),
            &[(
                "核对 pyvenv.cfg 的 home 字段和基础安装路径。",
                "Compare the home entry in pyvenv.cfg with the base installation.",
            )],
            &[VENV_DOC],
        )
    }
    .fact("environment", &info.kind)
    .private("sys.prefix", &info.prefix)
    .private("sys.base_prefix", &info.base_prefix);
    let pip_script = include_str!("../probes/pip.py");
    let pip_check=match probe(exe,pip_script,"--version",10) {
        Ok(out) if out.code==Some(0) && String::from_utf8_lossy(&out.stdout).starts_with("pip ")=>{
            Check::new("python.pip",Group::Python,pip,Status::Ok,("此解释器可以启动 pip。","This interpreter can launch pip.")).fact("pip",info.packages.get("pip").map(String::as_str).unwrap_or("available"))
        }
        Ok(_)=>Check::new("python.pip",Group::Python,pip,Status::Warning,("当前解释器无法启动 pip。","The selected interpreter could not launch pip."))
            .fact("pip_metadata",info.packages.contains_key("pip")).fact("pip_argv","--version")
            .explain(Confidence::Confirmed,("无法通过此解释器核对 pip 依赖。","pip dependency checks cannot run in this interpreter."),("pip 入口缺失或无法加载。","The pip entry point is absent or failed to load."),
                &[("核对解释器，再执行版本命令。便携环境请先查看其发行说明。","Verify the interpreter, then run the version command. Consult your distribution for portable environments.")],&[PIP_DOC])
            .command(ps_python(&requested,"-m pip --version")),
        Err(e)=>failed("python.pip",Group::Python,pip,"pip --version",e)
    };
    let mut dep_check = if pip_check.status != Status::Ok {
        skipped("python.deps", Group::Python, deps)
    } else {
        match probe(exe, pip_script, "check", 25) {
            Ok(out) if out.code == Some(0) => Check::new(
                "python.deps",
                Group::Python,
                deps,
                Status::Ok,
                (
                    "pip check 未报告依赖冲突。",
                    "pip check reported no broken requirements.",
                ),
            )
            .fact("pip_check_exit", 0),
            Ok(out) if out.code == Some(1) => {
                let lines = dependency_lines(&out.stdout);
                if lines.is_empty() {
                    failed(
                        "python.deps",
                        Group::Python,
                        deps,
                        "pip check",
                        Fault::Failed,
                    )
                } else {
                    let mut c=Check::new("python.deps",Group::Python,deps,Status::Warning,("pip check 报告缺少依赖或版本不满足要求。","pip check reported missing or incompatible requirements."))
                        .fact("pip_check_exit",1).explain(Confidence::Confirmed,("依赖这些包的工具可能无法启动。","Applications using these packages may fail to start."),("安装元数据中的要求与现有包不一致。","Installed packages do not satisfy declared requirements."),
                        &[("用下方命令复现，再对照项目的锁文件检查具体版本。","Reproduce with the command below, then compare versions against the project lockfile.")],&[PIP_DOC]).command(ps_python(&requested,"-m pip check"));
                    for l in lines {
                        c = c.fact("pip_check", l);
                    }
                    c
                }
            }
            Ok(_) => failed(
                "python.deps",
                Group::Python,
                deps,
                "pip check",
                Fault::Failed,
            ),
            Err(e) => failed("python.deps", Group::Python, deps, "pip check", e),
        }
    };
    for (pkg, ver) in &info.packages {
        dep_check = dep_check.fact(pkg, ver);
    }
    (Some(info), vec![runtime, environment, pip_check, dep_check])
}
fn dependency_lines(output: &[u8]) -> Vec<String> {
    // Extract package identifiers only. A requirement may contain a private URL.
    let valid = |s: &str| {
        !s.is_empty()
            && s.len() < 100
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    };
    String::from_utf8_lossy(output)
        .lines()
        .filter_map(|line| {
            let (left, right) = line
                .split_once(" has requirement ")
                .or_else(|| line.split_once(" requires "))?;
            let package = left.split_whitespace().next()?;
            let dependency = right
                .split(|c: char| !(c.is_ascii_alphanumeric() || "-_.".contains(c)))
                .next()?;
            (valid(package) && valid(dependency)).then(|| format!("{package} → {dependency}"))
        })
        .take(12)
        .collect()
}
