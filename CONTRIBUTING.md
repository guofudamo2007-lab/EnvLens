# Contributing / 参与开发

请从一条可复现的观测开始。反馈诊断问题时，请附 Windows 与 EnvLens 版本、解释器选择方式，以及已检查过的脱敏摘要。规则变更应说明哪些事实可以确认、哪些只是推测。添加新适配器时，提供离线样例，以及缺失、权限不足和无效输出等退化路径的测试。[开发说明](docs/DEVELOPMENT.md)

Start with a reproducible observation. For an incorrect finding, include Windows and EnvLens versions, the interpreter selection method and a reviewed, redacted export. Keep probes read-only, bound subprocess output and time, and record uncertainty. Add deterministic rule fixtures and include a Windows integration check when changing an OS adapter.

Before submitting a pull request, run the checks listed in the development guide. Keep translations and both user guides aligned with behavior changes. Contributions are accepted under the MIT license. [Development guide](docs/DEVELOPMENT.md)
