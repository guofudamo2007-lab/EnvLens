# Windows development / Windows 开发

## Prerequisites

Use a Windows x64 development machine with the Rust stable MSVC toolchain, Visual Studio’s **Desktop development with C++** workload and a Windows SDK. Install Node.js 24, the pnpm version pinned in package.json and WebView2. Python 3.8+ with the venv module is needed for the interpreter integration tests. An NVIDIA GPU and Ollama are optional. [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/)

以下命令在独立的 EnvLens 目录执行。现成安装包的用户无需安装开发工具。首次解析依赖、获取 NSIS 或缺少的 WebView2 运行时需要联网。

```powershell
pnpm install --frozen-lockfile
pnpm desktop
```

## Build and verify

```powershell
pnpm format:check
pnpm build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release -p envlens-core --bin envlens-scan --locked
pnpm bundle
```

The desktop installer is written under `target/release/bundle/nsis`. The portable scanner is `target/release/envlens-scan.exe`. The Windows workflow builds and uploads both on push, pull request and manual dispatch. A browser-only Vite page has no native probes. Run the desktop target to test real IPC.

`scripts/windows.ps1 -Task dev|build|test` also recognizes the optional workspace-local Rust toolchain in `.work`. It restores the shell environment on exit. Build tools are never installed by the application.

## Modules

| Path | Responsibility |
| --- | --- |
| `crates/diagnostics/src/windows.rs`, `system.rs` | Independent Windows version, CPU, memory, process and adapter observations |
| `runner.rs` | Absolute executables, independent argv, local working directory, bounded environment, job lifetime, deadline and output budget |
| `discovery.rs`, `python.rs`, `probes/` | Bounded discovery, trusted interpreter execution, package metadata and pip checks |
| `gpu.rs`, `torch.rs` | Adapter inventory, normalized nvidia-smi data, CUDA cause classification and optional tensor verification |
| `ollama.rs` | CLI, local processes, loopback-only reads, version and model-list validation |
| `model.rs`, `report.rs` | Bilingual observations, confidence, evidence, redaction and format rendering |
| `src-tauri/src/` | Approved target registry, single active scan, progress events, local history, native pickers, atomic file exports |
| `src/` | React desktop, navigation, language, themes, evidence view, history comparison and privacy preview |

The desktop accepts an interpreter only from the current discovery scope or a native file picker. It exposes typed scan and export commands. The frontend has event subscription capabilities. External links are restricted to the backend’s upstream documentation list. The Rust CLI invokes the same adapters without writing desktop history.

## Reproduce a report

```powershell
.\target\release\envlens-scan.exe --list
$python = (Get-Command python.exe).Source
.\target\release\envlens-scan.exe --python $python --format summary
.\target\release\envlens-scan.exe --project . --format md
.\target\release\envlens-scan.exe --python $python --verify-gpu --format json
```

`--list` prints local paths so you can choose the actual interpreter. The scanner defaults to redacted JSON. `--include-private` exposes original paths in that invocation. `--english` selects English prose in human-readable formats. A completed scan exits 0 even if findings exist. Invalid arguments exit 2. For comparisons, inspect check IDs, statuses and evidence values.

To cross-check a selected trusted environment in PowerShell, assign its full path to `$python` and run the following. Compare the results with the matching evidence fields. Activated shell settings or custom Python startup hooks may account for differences.

```powershell
& $python -c "import sys; print(sys.executable, sys.prefix, sys.base_prefix)"
& $python -m pip check
nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader,nounits
& $python -c "import torch; print(torch.__version__, torch.version.cuda, torch.cuda.is_available(), torch.cuda.device_count())"
Invoke-RestMethod 'http://127.0.0.1:11434/api/version'
Invoke-RestMethod 'http://127.0.0.1:11434/api/tags'
```

## Desktop acceptance test

The optional interactive Windows test needs Python packages `playwright` and `pywin32` in a developer test environment. It connects to the launched app’s WebView2 over a temporary loopback debugging port. It drives the native open/save dialogs, compares each export to its preview and checks that history reloads. It generates two real redacted test scans in the current user’s normal history. Use a test Windows account if you need an empty archive.

```powershell
py -m pip install playwright pywin32
py tests/desktop_smoke.py .\target\release\envlens.exe
```

Supply `--gpu-python <trusted-python.exe>` to select a CUDA environment, opt in to the one-element test, and require a successful tensor result. Supply `--screens docs/images` to capture the live overview and CUDA evidence. Diagnostic mock data is confined to tests. Desktop screenshots should be taken with on-screen privacy enabled.

## 发布准备

以此目录作为 GitHub 仓库根目录，CI 工作流位于 `.github/workflows/windows.yml`。公开发布时核对软件名称、签名和变更说明。当前产物未签名，GitHub Actions 的执行记录需在实际仓库中生成。

## Desktop interaction design

EnvLens uses Radix Toggle Group for keyboard-accessible theme and format choices. Status filters use native radio inputs with the same sliding indicator. The sliding indicator uses a CSS transform. The dashboard keeps its optical assembly in a local WebP. Pointer movement over the illustration schedules a single shared animation frame. Reduced motion switches off the movement and control transitions. Geist and the app's component licenses ship with the installer. [Radix Toggle Group](https://github.com/radix-ui/primitives/tree/main/packages/react/toggle-group) · [Frontend license texts](../public/THIRD-PARTY-NOTICES.txt)

The visual references were Obsidian Prism for damped response and optical edges, Dashral for the shared selection indicator, and Beszel for compact, aligned hardware readings. The optical illustration and CSS are original to EnvLens. Regenerate the illustration with `python scripts/render-optic.py` in an environment with NumPy and Pillow. [Obsidian Prism motion](https://github.com/RMvdesi/obsidian-prism/blob/main/motion-engine.js) · [Dashral segmented control](https://github.com/hasral/dashral/blob/main/packages/motion/src/components/segmented-control.tsx) · [Beszel system columns](https://github.com/henrygd/beszel/blob/main/internal/site/src/components/systems-table/systems-table-columns.tsx)

## Reproduce the UI timings

Start with one real archived report. Run the benchmark on each desktop executable using the same machine and report archive. Each output directory holds its own WebView profile, numeric samples and screenshot. The script opens and closes only the windows it starts.

```powershell
py tests/ui_benchmark.py .\target\release\envlens.exe --output .work\ui-benchmark
```
