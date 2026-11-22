use crate::{
    gpu::Gpu,
    model::*,
    python::{self, Python},
    runner::Fault,
    system::failed,
};
use serde::Deserialize;
use std::path::Path;
#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct Torch {
    pub installed: bool,
    pub version: Option<String>,
    pub cuda: Option<String>,
    pub hip: bool,
    pub xpu: bool,
    pub available: Option<bool>,
    pub count: Option<u32>,
    pub devices: Vec<String>,
    pub error: Option<String>,
    pub device_error: Option<String>,
    pub init_error: Option<String>,
    pub mask: bool,
    pub smoke: Option<bool>,
    pub smoke_error: Option<String>,
}
const INSTALL: Pair = ("PyTorch 安装", "PyTorch installation");
const CUDA: Pair = ("PyTorch CUDA 运行时", "PyTorch CUDA runtime");
const SMOKE: Pair = ("GPU 张量运算", "GPU tensor operation");
const VERIFY:&str="-c \"import torch; print(torch.__version__, torch.version.cuda, torch.cuda.is_available(), torch.cuda.device_count())\"";
pub fn collect(path: Option<&Path>, info: Option<&Python>, gpu: &Gpu, verify: bool) -> Vec<Check> {
    let (Some(exe), Some(info)) = (path, info) else {
        return skip_all(None);
    };
    match python::probe(
        exe,
        include_str!("../probes/torch.py"),
        if verify { "verify" } else { "inspect" },
        35,
    )
    .and_then(python::json::<Torch>)
    {
        Ok(t) => evaluate(
            &t,
            gpu,
            info.packages.get("torch").map(String::as_str),
            &exe.to_string_lossy(),
            verify,
        ),
        Err(e) => skip_all(Some(e)),
    }
}
fn skip_all(e: Option<Fault>) -> Vec<Check> {
    vec![
        e.map_or_else(
            || python::skipped("torch.install", Group::Torch, INSTALL),
            |e| failed("torch.install", Group::Torch, INSTALL, "import torch", e),
        ),
        python::skipped("torch.cuda", Group::Torch, CUDA),
        python::skipped("torch.smoke", Group::Torch, SMOKE),
    ]
}
fn below_family_floor(cuda: &str, driver: &str) -> bool {
    let floor = match cuda.split('.').next() {
        Some("11") => 450,
        Some("12") => 525,
        Some("13") => 580,
        _ => return false,
    };
    driver
        .split('.')
        .next()
        .and_then(|v| v.parse::<u32>().ok())
        .is_some_and(|d| d < floor)
}
pub fn evaluate(
    t: &Torch,
    gpu: &Gpu,
    metadata: Option<&str>,
    path: &str,
    verify: bool,
) -> Vec<Check> {
    if t.version.is_none() {
        let c = if !t.installed && metadata.is_none() {
            Check::new(
                "torch.install",
                Group::Torch,
                INSTALL,
                Status::Skipped,
                (
                    "当前环境未安装 PyTorch。",
                    "PyTorch is absent from this environment.",
                ),
            )
            .fact("distribution_present", false)
            .fact("import", "ModuleNotFoundError")
        } else {
            Check::new("torch.install",Group::Torch,INSTALL,Status::Error,("PyTorch 无法在当前解释器中加载。","PyTorch failed to import in this interpreter."))
                .fact("distribution",metadata.unwrap_or("unlisted")).fact("import_error",t.error.as_deref().unwrap_or("unknown"))
                .explain(Confidence::Undetermined,("此环境中的 PyTorch 工作负载无法启动。","PyTorch workloads cannot start in this environment."),("导入失败已确认，具体依赖或动态库仍需核对。","The import failed. The particular dependency or library still requires investigation."),
                &[("在相同解释器中运行导入命令以查看完整的本地错误。","Run the import in the same interpreter to see the full local error."),("对照官方支持的 Python 与 PyTorch 组合。","Compare Python and PyTorch versions with the official installer.")],&[TORCH_DOC]).command(ps_python(path,"-c \"import torch\""))
        };
        return vec![
            c,
            python::skipped("torch.cuda", Group::Torch, CUDA),
            python::skipped("torch.smoke", Group::Torch, SMOKE),
        ];
    }
    let installed = Check::new(
        "torch.install",
        Group::Torch,
        INSTALL,
        Status::Ok,
        (
            "当前解释器成功导入 PyTorch。",
            "The selected interpreter imported PyTorch.",
        ),
    )
    .fact("torch", t.version.as_deref().unwrap_or_default())
    .private("sys.executable", path);
    let bare = |s, msg| Check::new("torch.cuda", Group::Torch, CUDA, s, msg);
    let cause = |s, msg, confidence, why, step, docs: &[&str]| {
        bare(s, msg)
            .explain(
                confidence,
                (
                    "此解释器尚不能使用已验证的 NVIDIA 加速。",
                    "NVIDIA acceleration is unverified in this interpreter.",
                ),
                why,
                &[step],
                docs,
            )
            .command(ps_python(path, VERIFY))
    };
    let mut cuda = if t.cuda.is_none() {
        if gpu.nvidia == Some(true) {
            let why = if t.hip || t.xpu {
                (
                    "此 PyTorch 构建针对其他计算后端。",
                    "This PyTorch build targets a different accelerator backend.",
                )
            } else {
                (
                    "此 PyTorch 构建未包含 CUDA 支持。",
                    "This PyTorch build does not include CUDA support.",
                )
            };
            cause(Status::Warning,("当前 PyTorch 没有 CUDA 构建支持。","The current PyTorch build has no CUDA support."),Confidence::Confirmed,why,("核对所选解释器，在 PyTorch 官方安装选择器中选 Windows、Pip 和适配的 CUDA 构建。","Verify the interpreter. In the official installer select Windows, Pip and a suitable CUDA build."),&[TORCH_DOC])
        } else {
            bare(
                Status::Skipped,
                ("此构建未启用 CUDA。", "CUDA is not enabled in this build."),
            )
        }
    } else if t.available == Some(true) && t.count.unwrap_or(0) > 0 && t.device_error.is_none() {
        bare(
            Status::Ok,
            (
                "PyTorch 可枚举 CUDA 设备。",
                "PyTorch can enumerate CUDA devices.",
            ),
        )
    } else if t.mask {
        cause(
            Status::Warning,
            (
                "CUDA 设备可见性已被限制。",
                "CUDA device visibility is restricted.",
            ),
            Confidence::Confirmed,
            (
                "CUDA_VISIBLE_DEVICES 为空或为 -1。",
                "CUDA_VISIBLE_DEVICES is empty or set to -1.",
            ),
            (
                "核对应用启动时的设备可见性设置后再次检查。",
                "Review device visibility in the application launch environment and rescan.",
            ),
            &[TORCH_DOC],
        )
    } else if t.init_error.as_deref() == Some("insufficient_driver")
        || gpu
            .cards
            .iter()
            .any(|c| below_family_floor(t.cuda.as_deref().unwrap_or(""), &c.driver))
    {
        cause(Status::Error,("驱动与 CUDA 构建存在兼容性迹象。","The driver appears incompatible with this CUDA build."),Confidence::Likely,("驱动低于已知系列下限，或 CUDA 初始化报告驱动过旧。","The driver is below a known family floor, or initialization reported an insufficient driver."),("核对 NVIDIA 的驱动兼容表与 PyTorch 官方构建选项，然后复检。","Check NVIDIA’s driver compatibility table and PyTorch’s supported builds, then rescan."),&[CUDA_DOC,TORCH_DOC])
    } else if t.init_error.as_deref() == Some("driver_not_found") {
        cause(Status::Error,("PyTorch 初始化时未找到 NVIDIA 驱动。","PyTorch initialization could not find an NVIDIA driver."),Confidence::Likely,("此进程无法加载驱动。安装状态和访问权限仍需独立核对。","This process could not load the driver. Installation and access need independent verification."),("对照显示适配器及 nvidia-smi 的结果，核对设备管理器中的驱动状态。","Compare adapter inventory and nvidia-smi with the driver status in Device Manager."),&[GPU_DOC,TORCH_DOC])
    } else if t.init_error.is_some() || t.device_error.is_some() {
        cause(
            Status::Error,
            (
                "CUDA 设备初始化或枚举失败。",
                "CUDA device initialization or enumeration failed.",
            ),
            Confidence::Undetermined,
            (
                "已观察到运行时错误，尚不能确定根因。",
                "A runtime error was observed. Its root cause remains undetermined.",
            ),
            (
                "在相同解释器复现，结合设备名称与驱动证据排查。",
                "Reproduce in the same interpreter and compare device and driver evidence.",
            ),
            &[TORCH_DOC, CUDA_DOC],
        )
    } else {
        cause(
            Status::Unknown,
            (
                "CUDA 构建存在，设备可用性尚未确认。",
                "A CUDA build is present, but device availability is unconfirmed.",
            ),
            Confidence::Undetermined,
            (
                "当前结果不足以单独判定驱动、设备或环境故障。",
                "Current evidence does not isolate a driver, device or environment failure.",
            ),
            (
                "检查选定解释器、设备可见性和 nvidia-smi 结果并复检。",
                "Check the interpreter, device visibility and nvidia-smi results, then rescan.",
            ),
            &[TORCH_DOC],
        )
    };
    cuda = cuda.fact("torch.version.cuda", t.cuda.as_deref().unwrap_or("None"));
    if let Some(v) = t.available {
        cuda = cuda.fact("torch.cuda.is_available", v);
    }
    if let Some(n) = t.count {
        cuda = cuda.fact("device_count", n);
    }
    for name in &t.devices {
        cuda = cuda.fact("device_name", name);
    }
    if let Some(card) = gpu.cards.first() {
        cuda = cuda.fact("nvidia_driver", &card.driver);
    }
    cuda = cuda.fact("nvidia_smi", gpu.smi.map_or_else(|e| e.code(), |()| "ok"));
    if t.mask {
        cuda = cuda.fact("visibility", "empty or -1");
    }
    if let Some(e) = t.init_error.as_ref().or(t.device_error.as_ref()) {
        cuda = cuda.fact("runtime_error", e);
    }
    let smoke = match t.smoke {
        Some(true) => Check::new(
            "torch.smoke",
            Group::Torch,
            SMOKE,
            Status::Ok,
            (
                "首个 CUDA 设备完成单元素加法并读回 2。",
                "The first CUDA device returned 2 from one-element addition.",
            ),
        )
        .fact("device", "cuda:0")
        .fact("result", 2),
        Some(false) => Check::new(
            "torch.smoke",
            Group::Torch,
            SMOKE,
            Status::Error,
            ("实际张量运算失败。", "The tensor operation failed."),
        )
        .fact("operation", "ones(1).add_(1).item()")
        .fact(
            "error",
            t.smoke_error.as_deref().unwrap_or("unexpected_result"),
        )
        .explain(
            Confidence::Undetermined,
            (
                "此设备上的张量计算尚不可靠。",
                "Tensor computation on this device is unverified.",
            ),
            (
                "设备枚举成功，但计算或结果读回失败。",
                "Enumeration succeeded but computation or readback failed.",
            ),
            &[(
                "核对构建、设备架构及驱动后复检。",
                "Verify the build, GPU architecture and driver, then rescan.",
            )],
            &[CUDA_DOC, TORCH_DOC],
        ),
        None if !verify => Check::new(
            "torch.smoke",
            Group::Torch,
            SMOKE,
            Status::Skipped,
            (
                "本次未启用单元素 GPU 验算。",
                "The optional one-element GPU test was disabled.",
            ),
        ),
        None => python::skipped("torch.smoke", Group::Torch, SMOKE),
    };
    vec![installed, cuda, smoke]
}
