---
depends_on: []
---

# c95-add-bash-executor: 持久化 Bash 执行器（`!` / `!!` 命令 + 记录）

## Why
pi 的 `core/bash-executor.ts`（156 LOC）配合 AgentSession 的 `executeBash` / `recordBashResult` / `abortBash`，支持交互/RPC 模式下用户直接执行 `!cmd`（结果入上下文）和 `!!cmd`（结果不入上下文）。xylitol 的 `src/agent/tools/bash.rs` 只服务 LLM tool-call，无法满足交互/RPC 场景下的直接 shell 执行与记录。

## What Changes
- 新增 `src/agent/bash_executor.rs::BashExecutor`：
  - `execute(cmd, opts) -> BashResult { output, exit_code, cancelled, truncated, full_output_path }`
  - `on_chunk` 流式回调（已清洗 ANSI）
  - AbortSignal 取消 → 复用 `tools/process.rs::kill_tree` 杀进程组
  - 输出截断到 `DEFAULT_MAX_BYTES`，超限 spill 到临时文件（复用 `tools/truncate.rs`）
- `SessionEntry`（`infra/session/types.rs`）增加 `BashExecution { command, output, exit_code, cancelled, truncated, full_output_path, exclude_from_context }` 变体
- `AgentSession` 增 `execute_bash()` / `record_bash_result()` / `abort_bash()`，并在构建 LLM 上下文时过滤 `exclude_from_context == true` 的 BashExecution 条目
- `process_prompt` 拦截以 `!` / `!!` 开头的输入路由到 executor

## Capabilities
- bash-execution

## Impact
- `SessionEntry` 增变体：serde 向后兼容（新变体有默认反序列化），无需迁移。
- 复用现有 `tools/bash.rs` 的 kill_tree / accumulator / truncate，不重写 tool 版本。
- 触及文件：`src/agent/bash_executor.rs`（新增）、`src/agent/session.rs`（方法）、`src/infra/session/types.rs`（变体）、`src/agent/mod.rs`（导出）。
