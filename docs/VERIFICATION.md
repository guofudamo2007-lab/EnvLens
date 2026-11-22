# EnvLens 验证记录 / Verification record

## 0.1.1 Windows 桌面

2026-11-23，在同一台 Windows 11 x64 设备、相同 WebView2 和同一份已脱敏归档上比较 0.1.0 与 0.1.1。两个版本交替运行，次序为旧、新、新、旧。每版合计 10 次独立进程启动、20 次桌面页面热重载和 60 次页签切换。显示器和 WebView 已预热。下表单位均为毫秒，每格为中位数 / P95。

| 实测项目 | 0.1.0 | 0.1.1 |
| --- | ---: | ---: |
| 启动进程到同时观察到报告且扫描可用 | 821.7 / 877.7 | 692.5 / 742 |
| 热重载到历史报告进入页面 | 167 / 182.2 | 40 / 49.7 |
| 热重载到解释器选择就绪 | 167 / 182.2 | 173.4 / 212.8 |
| 切页点击到第二个动画帧回调 | 8.3 / 16.5 | 8.4 / 21.6 |

进程启动指标包含本机自动化连接和观察开销。切页指标测量浏览器调度，完整选中动画会继续到过渡结束。扫描探针的执行时长取决于所选 Python、驱动和服务。本机两轮新版总览在静止后记录到 0 个运行中的页面动画。[复现脚本](../tests/ui_benchmark.py)

安装版的桌面验证覆盖 18 项真实检查、GPU 加法读回 2、原生文件选择、三种导出与预览逐字一致、剪贴板脱敏、历史恢复与比较。界面测试还检查了方向键选中与焦点、状态筛选联动、硬件证据、深浅色、中英文、800×600 布局以及减少动态效果。Rust 工作区 39 项测试通过，Clippy 和前端构建通过。[桌面测试](../tests/desktop_smoke.py)

## 0.1.0 原始验收记录

验证日期为 2026-11-22。以下记录来自 Windows 11 x64 本机。CI 配置面向 Windows GitHub-hosted runner，线上执行记录需在发布仓库中生成。

## 本机观测

| 项目 | 结果 |
| --- | --- |
| 基础环境 | Windows 11 x64，AMD Ryzen 9 8945HX，32 个逻辑处理器 |
| NVIDIA | GeForce RTX 5070 Laptop GPU，驱动 617.14，nvidia-smi 返回 8151 MiB |
| 解释器选择 | 系统发现的 Python 与独立测试 venv 分别检查，空项目未回退到全局 Python |
| PyTorch | 测试 venv 中为 2.11.0+cu128，CUDA 构建 12.8，可用，1 个设备 |
| 实际 GPU 计算 | 用户启用验算后，cuda:0 上的单元素加法读回 2 |
| Ollama | 隔离启动官方 Windows 0.40.17，识别 CLI 和进程，服务返回 0.40.17，清单成功返回 0 个模型，推理显示未检查 |
| 开发与打包 | Vite + Tauri 开发窗口与 Rust IPC，生产前端、Rust 发布构建及 NSIS x64 安装程序 |

硬件和运行时的完整字段见脱敏样例。Ollama 进程只在验收时运行，未下载模型。[真实 JSON 样例](examples/windows-cuda.json)

## 自动与桌面检查

39 项 Rust 测试全部通过，Clippy 在禁止警告模式下通过。安装程序清单的执行级别为 `asInvoker`，当前用户安装后的二进制与构建文件逐字节核对，差异仅为 Tauri 写入的 NSIS 类型标记。安装版桌面检查覆盖 1024 × 720 和 800 × 600 的内容视口。

Rust 测试覆盖驱动与构建判定、CUDA 可见性、初始化失败、可选计算、多个解释器来源、UTF-16 Launcher 输出、真实 venv 中的依赖冲突、退出码、权限、超时、输出量、参数传递、报告转义和历史归档。Windows 集成测试使用真实系统接口。Ollama 的畸形响应使用独立回环端口的测试服务。

桌面验收在 WebView2 上通过原生对话框选择解释器、项目及保存路径。用实际导出文件逐字核对 Markdown、JSON 和 HTML 预览。检查了主题、中英文界面、筛选、证据、摘要复制、历史重载和状态对比。[复现方法](DEVELOPMENT.md)

## 后续验证范围

Windows 10 独立设备、不同代际 NVIDIA 驱动、多 GPU 和不同 Conda 发行版仍需更多实机覆盖。当前发布包没有 Authenticode 签名。当前版本不会执行 Ollama 模型推理，驱动下限规则也不保证所有计算特性兼容。

On Windows 11 x64 the real CUDA test read back 2, and an isolated official Ollama server returned an empty model list. The local Windows test suite and native desktop workflow can be rerun using the development guide. Windows 10 hardware coverage, additional drivers and signed distribution remain open.
