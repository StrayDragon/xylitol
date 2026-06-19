# Design: Bash Executor

对齐 pi `core/bash-executor.ts` + AgentSession 的 `executeBash/recordBashResult/abortBash`。

## 决策

1. **复用而非重写 tool 版 bash**：`tools/bash.rs` 是 LLM tool-call 入口（单次执行）；`BashExecutor` 是用户/RPC 直执行入口（带记录）。两者共享 `tools/process.rs::kill_tree`、`tools/accumulator.rs`、`tools/truncate.rs`，不合并执行路径。
2. **SessionEntry 新增 `BashExecution` 变体**：serde 向后兼容（新变体可默认反序列化为 None/跳过）。携带 `exclude_from_context` 区分 `!`（入上下文）/`!!`（不入）。
3. **前缀路由**：`process_prompt` 检测行首 `!!` → exclude=true，单个 `!` → exclude=false；剥离前缀后交 executor。注意 `!!` 必须先于 `!` 判定。
4. **上下文过滤点**：在构建发送给 LLM 的历史处统一过滤 `exclude_from_context==true` 的 BashExecution，单点实现，避免多处漏过滤。
5. **取消**：复用 CancellationToken + kill_tree 杀进程组；`abort_bash()` 触发 token。

## 不做

- 不实现持久化 bash 会话（环境变量跨命令复用）；pi 亦非持久 shell，每次 `sh -c`。
- 不接入 RPC（RPC 的 bash 命令由 c115 调用 `execute_bash`，本变更只提供能力）。
