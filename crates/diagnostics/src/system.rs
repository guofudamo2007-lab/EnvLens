use crate::{model::*, runner::Fault};
pub struct Os {
    pub name: String,
    pub version: String,
    pub arch: String,
}
pub struct Memory {
    pub total_kib: u64,
    pub free_kib: u64,
}
pub struct Cpu {
    pub name: String,
    pub threads: u32,
}
pub struct Adapter {
    pub name: String,
    pub nvidia: bool,
    pub wddm_driver: Option<String>,
}
pub struct Host {
    pub os: Result<Os, Fault>,
    pub memory: Result<Memory, Fault>,
    pub cpu: Result<Cpu, Fault>,
    pub adapters: Result<Vec<Adapter>, Fault>,
    pub ollama: Result<bool, Fault>,
}
#[cfg(windows)]
#[path = "windows.rs"]
mod native;
#[cfg(windows)]
pub use native::collect;
#[cfg(not(windows))]
pub fn collect() -> Host {
    Host {
        os: Err(Fault::Missing),
        memory: Err(Fault::Missing),
        cpu: Err(Fault::Missing),
        adapters: Err(Fault::Missing),
        ollama: Err(Fault::Missing),
    }
}
pub fn failed(id: &str, group: Group, title: Pair, source: &str, e: Fault) -> Check {
    let summary = match e {
        Fault::Denied => (
            "系统拒绝了探针访问。",
            "The operating system denied this probe.",
        ),
        Fault::Timeout => (
            "探针超过时间限制，结果未知。",
            "The probe exceeded its deadline.",
        ),
        Fault::OutputLimit => (
            "探针输出超过限制，结果未知。",
            "The probe exceeded its output limit.",
        ),
        Fault::Missing => (
            "在检查范围内未找到所需程序。",
            "The required executable was not found in scope.",
        ),
        _ => (
            "探针未返回可验证的数据。",
            "The probe did not return verifiable data.",
        ),
    };
    Check::new(id, group, title, Status::Unknown, summary)
        .fact("probe", source)
        .fact("probe_result", e.code())
        .explain(
            Confidence::Undetermined,
            ("本项能力尚未验证。", "This capability remains unverified."),
            summary,
            &[(
                "在同一用户会话中核对探针和访问权限，再次运行体检。",
                "Check the probe and its access in the same user session, then run the scan again.",
            )],
            &[],
        )
}
pub fn checks(h: &Host) -> Vec<Check> {
    let os = ("操作系统", "Operating system");
    let cpu = ("处理器", "Processor");
    let memory = ("物理内存", "Physical memory");
    vec![
        match &h.os {
            Ok(o) => Check::new(
                "system.os",
                Group::System,
                os,
                Status::Ok,
                (
                    "已读取系统版本与架构。",
                    "Read the OS version and architecture.",
                ),
            )
            .fact("os", &o.name)
            .fact("version", &o.version)
            .fact("architecture", &o.arch),
            Err(e) => failed("system.os", Group::System, os, "Windows CurrentVersion", *e),
        },
        match &h.cpu {
            Ok(c) => Check::new(
                "system.cpu",
                Group::System,
                cpu,
                Status::Ok,
                (
                    "已读取 CPU 型号和逻辑处理器数量。",
                    "Read CPU model and logical processor count.",
                ),
            )
            .fact("cpu", &c.name)
            .fact("threads", c.threads),
            Err(e) => failed(
                "system.cpu",
                Group::System,
                cpu,
                "Windows processor inventory",
                *e,
            ),
        },
        match &h.memory {
            Ok(o) if o.total_kib > 0 && o.free_kib <= o.total_kib => Check::new(
                "system.memory",
                Group::System,
                memory,
                Status::Ok,
                (
                    "采样时的系统可见内存。",
                    "System-visible memory at scan time.",
                ),
            )
            .fact("total_mib", o.total_kib / 1024)
            .fact("available_mib", o.free_kib / 1024),
            _ => failed(
                "system.memory",
                Group::System,
                memory,
                "GlobalMemoryStatusEx",
                h.memory.as_ref().err().copied().unwrap_or(Fault::Invalid),
            ),
        },
    ]
}
