---
depends_on:
  - c95-add-bash-executor
  - c100-add-prompt-templates
  - c105-add-export-capabilities
---

# c110-update-slash-commands: Slash 命令全集 + source + 派发

## Why
pi 有 22 个 builtin slash command，每条带 `source: extension|prompt|skill` + `SourceInfo`，并由 AgentSession 派发到具体处理器。xylitol 的 `src/agent/commands.rs` 只有 7 条无 source 的元数据常量，无 `SlashCommandSource`、无 `SourceInfo`、无派发逻辑。本变更重写命令系统，并把 `/export`→c105、`!`/`!!`→c95、`/prompt`→c100 接入派发。TUI 专属命令（settings/scoped-models/changelog/hotkeys/trust UI）在本期返回"需 TUI，暂不可用"提示。

## What Changes
- 重写 `src/agent/commands.rs`：
  - `SlashCommandInfo { name, description, source: SlashCommandSource, source_info: SourceInfo }`
  - `enum SlashCommandSource { Extension, Prompt, Skill }`
  - 22 条 builtin 表（对齐 pi：settings/model/scoped-models/export/import/share/copy/name/session/changelog/hotkeys/fork/clone/tree/trust/login/logout/new/compact/resume/reload/quit）
  - `AgentSession::dispatch_slash_command(name, args) -> DispatchResult`：路由到 handler
- TUI 专属命令在 `dispatch_slash_command` 中返回 `NotAvailable { reason: "需要 TUI" }`
- `process_prompt` 改为先尝试 `dispatch_slash_command`，再走正常 LLM 流程
- 按 AGENTS.md「不做 BC shim」一次性替换旧 `SlashCommandInfo` / `BUILTIN_COMMANDS`

## Capabilities
- cli-entry

## Impact
- **破坏性**：移除旧 `SlashCommandInfo`（7 字段常量版本）→ 新结构。所有调用方（session.rs / 测试）一次性更新。
- 触及文件：`src/agent/commands.rs`（重写）、`src/agent/session.rs`（dispatch + process_prompt）、调用方与测试。
