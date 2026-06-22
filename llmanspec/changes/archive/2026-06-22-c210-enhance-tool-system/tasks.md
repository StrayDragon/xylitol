# c210-enhance-tool-system: Tasks

## Tool Filtering

- [x] 在 ToolRegistry 中添加 `set_allowed_tool_names()` / `set_excluded_tool_names()`
- [x] 添加 `list_filtered()` — 尊重 allow 优先 + deny 叠加
- [x] 原有 `list()` 保持不变（返回所有工具）

## Argument Validation

- [x] 添加 `validate_tool_arguments(tool: &dyn XyTool, args: &Value)` 函数
- [x] 校验：必填字段、类型匹配、意外字段
- [x] 返回 `ToolValidationError` 枚举（MissingField / TypeError / UnexpectedField）

## Per-Tool Execution Mode

- [x] 向 XyTool trait 添加 `execution_mode() -> ToolExecutionMode`（默认：Parallel）
- [x] EditTool 标记为 Sequential
- [x] AgentLoop 并行检查：if any tool is Sequential → fallback to sequential

## ToolDefinition Wrapper

- [x] 定义 `ToolDefinition` 结构体（name, description, parameters, prompt_snippet, prompt_guidelines, execution_mode, source_info）
- [x] 实现 `From<&dyn XyTool> for ToolDefinition`
- [x] `prompt_snippet()` 默认回退到 description 前 80 字符
- [x] `prompt_guidelines()` 标准化为 `Vec<String>`
- [x] 添加 `prepare_arguments(args: Value) -> Value` 到 XyTool trait（默认：identity）

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — 工具系统测试通过
- [x] `llman sdd validate c210-enhance-tool-system`
