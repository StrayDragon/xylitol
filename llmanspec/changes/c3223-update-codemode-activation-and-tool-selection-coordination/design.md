# Design — codemode 激活与工具选择协调

## 目标

- 让 `codemode.enabled` 与 `tools` 选择有一致的、可诊断的语义。
- 避免“插件已启用但工具被禁用”仍启动 Wasmtime。
- 避免“工具被选中但插件未启用”静默失败。

## 现状

- `optional_plugin_rows` 在 `codemode.is_active()` 时 push 插件行。
- `CodemodePlugin::apply` 立即创建 `CodemodeRunner`、注册 `run_code`、注册声明中间件。
- `compose_tools` / `resolve_tool_selection` 在插件 apply 之后执行 `tools` 的 `+` / `-`。
- `tools` 空列表表示“全部启用”；非空条目是修饰，不是 allowlist。
- `+run_code` 若 `run_code` 不在工具集，`resolve_tool_selection` 报 unknown tool warning。

## 决策

### 1. 激活与选择分层

- `codemode.enabled` 是唯一激活开关；`tools` 不得隐式激活插件。
- `tools: [+run_code]` 但插件未启用：启动日志 / 配置检查 warning，明确“codemode disabled; run_code selection ignored”。
- `codemode.enabled=true` 且 `-run_code`：warning“codemode enabled but run_code disabled”；插件可注册，但不得创建 engine。
- 不把 `tools` 改成 allowlist；保持现有 `+` / `-` 语义。

### 2. 惰性 runner

- `RunCodeTool` 持有 `LazyRunner`（`OnceLock<Arc<CodemodeRunner>>` 或等价）。
- 首次 `execute` 才创建 `CodemodeRunner`；创建失败返回 `ExecutionFailed`，不缓存失败。
- `CodemodePlugin::apply` 只注册工具、设置 policy / max nested、注册声明中间件；不创建 engine。
- 未选择 `run_code` 或从未调用时，零 Wasmtime engine / 零 guest 编译。

### 3. 配置校验

- 在 boot 读取 `codemode` 与 `tools` 后做跨 section 检查；warning 走现有日志 / 诊断面。
- 不因 warning 阻断启动；除非未来决定把矛盾配置升级为 error。
- `config check` 可复用同一检查；不得只在 TUI 显示。

### 4. 声明与 catalog

- 声明仍由 c3218 从选择后的 catalog 生成；本 change 不重复实现。
- `run_code` 未选择时不出现在声明 / model schemas；插件存在但不可见。

## 数据流

config: codemode.enabled / tools +/-
  -> optional_plugin_rows (activation)
  -> CodemodePlugin::apply (register lazy run_code, policy, declarations)
  -> compose_tools -> resolve_tool_selection
  -> model tool schemas / catalog
  -> run_code execute -> LazyRunner -> Wasmtime

## Seams 与测试

- 单测：disabled -> 无 run_code、无 engine。
- 单测：enabled + `-run_code` -> 有 warning，无 engine；首次调用不可达。
- 单测：enabled + selected -> 首次 execute 才建 engine；第二次复用。
- 单测：`+run_code` 未启用 -> warning。
- 回归：`tools` 空列表 / `+` / `-` 语义不变。

## 风险

- 惰性创建把失败从 boot 推迟到首次调用；需要清晰错误。
- 跨 section warning 可能被忽略；要进入现有诊断面。
- 插件仍注册但不可见时，声明中间件不应列出 run_code；与 c3218 协同。

## 未决问题

- warning 面：boot log / `config check` / TUI 诊断，是否需要结构化 warning。
- 惰性 runner 创建失败是否重试；配置校验是否在 boot 先做。
- `run_code` 被禁用时插件是否仍注册声明中间件；与 c3218 的 catalog 空态协同。

## 迁移

- 不改变配置 schema 字段；只增加校验与惰性行为。
- 不改变 run_code 的权限、隔离、guest ABI。
