use crate::{
    model::*,
    runner::{self, Fault},
    system::{failed, Adapter, Host},
};
use std::{env, path::PathBuf};
#[derive(Clone, Debug)]
pub struct Card {
    pub name: String,
    pub driver: String,
    pub memory_mib: Option<u64>,
}
pub struct Gpu {
    pub cards: Vec<Card>,
    pub nvidia: Option<bool>,
    pub smi: Result<(), Fault>,
}
pub fn parse_smi(bytes: &[u8]) -> Result<Vec<Card>, Fault> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .trim(csv::Trim::All)
        .from_reader(bytes);
    let mut cards = vec![];
    for row in rdr.records().take(64) {
        let r = row.map_err(|_| Fault::Invalid)?;
        if r.len() != 3
            || r[0].is_empty()
            || !r[1]
                .split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(Fault::Invalid);
        }
        cards.push(Card {
            name: r[0].into(),
            driver: r[1].into(),
            memory_mib: r[2].parse().ok(),
        });
    }
    if cards.is_empty() {
        return Err(Fault::Invalid);
    }
    Ok(cards)
}
fn smi() -> Result<(Vec<Card>, PathBuf), Fault> {
    let mut paths = vec![];
    if let Some(p) = runner::system32("nvidia-smi.exe") {
        paths.push(p);
    }
    if let Some(base) = env::var_os("ProgramFiles") {
        paths.push(PathBuf::from(base).join(r"NVIDIA Corporation\NVSMI\nvidia-smi.exe"));
    }
    paths.extend(runner::path_exes("nvidia-smi.exe"));
    let exe = paths
        .into_iter()
        .find(|p| p.is_file())
        .ok_or(Fault::Missing)?;
    let out = runner::run(
        &exe,
        &[
            "--query-gpu=name,driver_version,memory.total",
            "--format=csv,noheader,nounits",
        ],
        std::time::Duration::from_secs(8),
    )?;
    match out.code {
        Some(0) => Ok((parse_smi(&out.stdout)?, exe)),
        Some(4) => Err(Fault::Denied),
        Some(9) => Err(Fault::DriverNotLoaded),
        _ => Err(Fault::Failed),
    }
}
pub fn collect(host: &Host) -> (Gpu, Vec<Check>) {
    let hardware = ("显示适配器", "Display adapters");
    let driver = ("NVIDIA 驱动与显存", "NVIDIA driver and VRAM");
    let seen = host
        .adapters
        .as_ref()
        .ok()
        .map(|a| a.iter().any(|d| d.nvidia));
    let hw = match &host.adapters {
        Ok(adapters) if !adapters.is_empty() => {
            let mut c = Check::new(
                "gpu.adapters",
                Group::Gpu,
                hardware,
                Status::Ok,
                (
                    "Windows 枚举到以下显示设备。",
                    "Windows enumerated these display devices.",
                ),
            );
            for Adapter {
                name, wddm_driver, ..
            } in adapters
            {
                c = c.fact("adapter", name);
                if let Some(v) = wddm_driver {
                    c = c.fact("windows_driver", v);
                }
            }
            c.fact("nvidia_detected", seen == Some(true))
        }
        _ => failed(
            "gpu.adapters",
            Group::Gpu,
            hardware,
            "EnumDisplayDevicesW",
            host.adapters
                .as_ref()
                .err()
                .copied()
                .unwrap_or(Fault::Invalid),
        ),
    };
    let (gpu, check) = match smi() {
        Ok((cards, path)) => {
            let mut c = Check::new(
                "gpu.driver",
                Group::Gpu,
                driver,
                Status::Ok,
                (
                    "nvidia-smi 已与驱动通信。",
                    "nvidia-smi communicated with the driver.",
                ),
            )
            .private("nvidia_smi", path.display());
            for card in &cards {
                c = c.fact("gpu", &card.name).fact("driver", &card.driver);
                if let Some(m) = card.memory_mib {
                    c = c.fact("vram_mib", m);
                }
            }
            (
                Gpu {
                    cards,
                    nvidia: Some(true),
                    smi: Ok(()),
                },
                c,
            )
        }
        Err(e) => {
            let c = if seen == Some(false)
                && host.adapters.as_ref().is_ok_and(|a| !a.is_empty())
                && e == Fault::Missing
            {
                Check::new(
                    "gpu.driver",
                    Group::Gpu,
                    driver,
                    Status::Skipped,
                    (
                        "Windows 未列出 NVIDIA 设备。",
                        "Windows listed no NVIDIA device.",
                    ),
                )
                .fact("nvidia_smi", e.code())
            } else if e == Fault::DriverNotLoaded {
                Check::new(
                    "gpu.driver",
                    Group::Gpu,
                    driver,
                    Status::Error,
                    (
                        "nvidia-smi 报告 NVIDIA 驱动未加载。",
                        "nvidia-smi reported that the NVIDIA driver is not loaded.",
                    ),
                )
                .fact("nvidia_smi_exit", 9)
                .explain(
                    Confidence::Confirmed,
                    (
                        "需要该驱动的 NVIDIA 加速不可用。",
                        "NVIDIA acceleration requiring this driver is unavailable.",
                    ),
                    (
                        "工具返回 NVIDIA 定义的驱动未加载状态。",
                        "The utility returned NVIDIA's driver-not-loaded status.",
                    ),
                    &[(
                        "查看设备管理器中的驱动状态，再次运行 nvidia-smi。",
                        "Check the driver status in Device Manager and rerun nvidia-smi.",
                    )],
                    &[GPU_DOC],
                )
                .command("nvidia-smi".into())
            } else if seen == Some(true) && matches!(e, Fault::Missing | Fault::Failed) {
                Check::new(
                    "gpu.driver",
                    Group::Gpu,
                    driver,
                    Status::Warning,
                    (
                        "已发现 NVIDIA 设备，驱动通信尚未验证。",
                        "An NVIDIA adapter is visible, but driver communication is unverified.",
                    ),
                )
                .fact("nvidia_detected", true)
                .fact("nvidia_smi", e.code())
                .explain(
                    Confidence::Likely,
                    (
                        "CUDA 程序可能无法访问此设备。",
                        "CUDA applications may be unable to access the device.",
                    ),
                    (
                        "工具缺失或驱动未响应，单凭此结果无法区分。",
                        "The utility may be missing or the driver may not be responding.",
                    ),
                    &[
                        (
                            "在设备管理器中查看该适配器的设备状态。",
                            "Inspect the adapter status in Device Manager.",
                        ),
                        (
                            "从终端运行 nvidia-smi，再复检。",
                            "Run nvidia-smi in a terminal and repeat the scan.",
                        ),
                    ],
                    &[GPU_DOC],
                )
                .command("nvidia-smi".into())
            } else {
                failed("gpu.driver", Group::Gpu, driver, "nvidia-smi", e)
            };
            (
                Gpu {
                    cards: vec![],
                    nvidia: seen,
                    smi: Err(e),
                },
                c,
            )
        }
    };
    (gpu, vec![hw, check])
}
