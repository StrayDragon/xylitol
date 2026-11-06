# Tasks

## 1. 激活与选择

- [x] 1.1 `ToolFeatures` 增加 `codemode_enabled`（来自 `codemode.enabled`）；`compose_tools` 对“`+run_code` 但未启用”记录启动 warning；`enabled + -run_code` 不告警。
- [x] 1.2 `config check` 识别“`run_code` 被选择但 codemode 未启用”并输出 warning，不阻断启动。
- [x] 1.3 保持 `tools` 现有 `+` / `-` 语义；不改 allowlist，不隐式激活插件。

## 2. 惰性 runner

- [x] 2.1 `RunCodeTool` 持有 `CodemodeConfig` 与惰性 runner；首次 `execute_outcome` 才创建 `CodemodeRunner`。
- [x] 2.2 创建失败返回 `ExecutionFailed`，不缓存失败；第二次调用重试。
- [x] 2.3 `CodemodePlugin::apply` 不创建 engine；只注册 `run_code`、policy / max nested、声明中间件。
- [x] 2.4 `-run_code` 或从未调用时零 Wasmtime engine / 零 guest 编译。

## 3. 测试与规格

- [x] 3.1 单测：disabled 零装配；enabled + `-run_code` 合法且无 warning；首次调用才初始化 runner、第二次复用；创建失败可重试。
- [x] 3.2 BDD：`+run_code` 未启用 warning；`enabled + -run_code` 合法；`run_code` 未选择时不出现在工具集。
- [x] 3.3 回归：`tools` 空列表 / `+` / `-` 语义不变；其他工具显示与行为不变。

## 4. 门禁

- [x] 4.1 `just fmt`、相关 clippy、`just test`、`just qa`；BDD 新场景绑定并全绿。
