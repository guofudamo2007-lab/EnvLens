# Reading a diagnosis

[简体中文](USAGE.zh-CN.md) · [Home](../README.en.md)

## Select the application’s interpreter

Discovery checks active venv and Conda prefixes, absolute PATH entries, Python Launcher and the user’s Conda environments registry. Duplicate paths retain their discovery sources. Choose another candidate, browse to a trusted local Python executable or choose a project folder. A project scan checks only its conventional root, venv and portable interpreter locations. An empty project produces a scope finding and skips Python checks. Browse to the interpreter for an externally managed environment.

The Python page labels the next scan target separately from the interpreter in the displayed report. Changing the target requires a new scan.

## Interpret the evidence

| State | Meaning |
| --- | --- |
| Passed | The named observation or operation succeeded |
| Warning | Evidence warrants review in this workflow |
| Error | The probe observed a failed operation or compatibility obstacle |
| Skipped | Optional, disabled or lacking a prerequisite |
| Unknown | Incomplete evidence, access failure, timeout, output limit or invalid response |

A finding includes impact, evidence, confidence, possible cause, verification steps and applicable upstream documentation. Inspect the complete local error with the suggested verification command if a compact error category is insufficient.

For example, when Windows sees an NVIDIA GPU but a project runs on the CPU, first compare the selected interpreter with the project launch command. Then inspect the driver query and PyTorch build. A value of `None` for `torch.version.cuda` establishes that the build lacks CUDA support. Use the official installer selector if you want GPU acceleration in that interpreter. Rescan with the optional one-element tensor test. A result of 2 verifies the calculation and readback on the first CUDA device. CUDA initialization may allocate runtime memory. [PyTorch installer](https://pytorch.org/get-started/locally/) · [NVIDIA compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/latest/minor-version-compatibility.html)

## Share or compare

Copy Summary always masks private paths. Review the Markdown, JSON or standalone HTML preview before saving. Disable redaction only if the recipient needs original paths or model names. History always stores a redacted snapshot. The latest 20 snapshots are kept under `%LOCALAPPDATA%\dev.envlens.localdiagnostics\history-v1\`. Compare two scans by check ID to see status changes. Export JSON for a detailed environment comparison. Settings can mask paths on screen for presentations.

## Operating limits

- The deep probes target CPython 3.8+. Isolated mode disables project imports, executable .pth lines and site customization. Environments dependent on editable-install hooks or special DLL search paths can differ from an activated shell. The selected interpreter and installed PyTorch package must be trusted.
- Driver family floors identify clear compatibility concerns. A newer driver alone does not verify device architecture or all CUDA features. Optional computation is limited to device zero.
- Ollama is inspected at 127.0.0.1:11434 with GET requests to the version and tags endpoints. A maximum of 40 model names is shown with the total count. Model inference is skipped. [Ollama model list](https://docs.ollama.com/api/tags)
- This release is unsigned. Development dependency installation and provisioning a missing WebView2 runtime can require internet access.
