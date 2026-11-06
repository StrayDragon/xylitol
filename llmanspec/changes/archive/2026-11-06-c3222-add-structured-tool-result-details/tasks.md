# Tasks

## 1. SDK 结果类型

- [x] 1.1 新增 `XyToolErrorKind`（snake_case）与 `XyToolOutcome`（parts / details / is_error / error_kind）；导出到 SDK 根。
- [x] 1.2 `XyTool` 改为 `execute_outcome`；删除 `execute_as_parts` / `XyToolCallResult` 旧路径；保留 `execute` 文本便捷适配；提供 text outcome 辅助构造。
- [x] 1.3 迁移全部工具实现（core tools、todo、workflow、ask、MCP、codemode、外进程插件、测试工具）到 `execute_outcome`。
- [x] 1.4 嵌套 `DispatchHandle` 与 outer tool 共用 `XyToolOutcome`；dispatch 测试覆盖 details / error_kind。

## 2. Runtime 事件与 wire

- [x] 2.1 `ToolExecutionEnd` 增加 `details: Option<Value>`；wire 映射透传，不 bump `PROTOCOL_VERSION`。
- [x] 2.2 `run_one` 使用 `execute_outcome`；after_tool / hook 修改时同步更新 details、is_error、error_kind。
- [x] 2.3 `AgentMessage` 与事件使用同一份 details；不再只从 parts 推导。
- [x] 2.4 单测：details 透传、after_tool 替换、缺失 details 回退、error_kind 映射。

## 3. run_code details

- [x] 3.1 `RunCodeTool` 返回 `XyToolOutcome`；parts 保持脚本结果文本。
- [x] 3.2 host dispatch 收集嵌套调用摘要（name / args_preview / ok / error_kind / elapsed_ms）；runner 生成 `script_summary` / `elapsed_ms` / `truncated` / `output_bytes`。
- [x] 3.3 错误映射：timeout / cancelled / script / runtime / invalid_args；保留可读消息。
- [x] 3.4 单测：嵌套摘要、截断、错误类别、无嵌套调用。

## 4. TUI 消费

- [x] 4.1 bridge 接收 `ToolExecutionEnd.details`；layout / widgets 只读消费。
- [x] 4.2 `run_code` 外层显示脚本摘要、结果、错误类别、截断标记、嵌套摘要；无 details 回退文本。
- [x] 4.3 harness 测试：有 details / 无 details、成功 / 错误 / 截断 / 嵌套摘要。

## 5. 规格与门禁

- [x] 5.1 `agent-tools` 新增规则：工具结果携带结构化 details 与 error_kind；场景覆盖 `run_code` 成功 details、错误类别、缺失 details 回退。
- [x] 5.2 更新受影响的 SDK / runtime 文档或 AGENTS 指针；不复制实现细节。
- [x] 5.3 `just fmt`、相关 clippy、`just test`、`just qa`；BDD 新场景绑定并全绿。
