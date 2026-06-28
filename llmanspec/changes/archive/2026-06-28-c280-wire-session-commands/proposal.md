---
depends_on:
  - c278-slim-agent-session-and-merge-export
---

# c280-wire-session-commands

> **状态**：draft 提案（2026-06-27）。解耦路线图收尾后的"兑现红利"变更——把 c278 铺好的
> SessionStore port 接到真实的会话命令通道，并补齐 AGENTS.md / 系统提示 / skills
> 在 live 系统提示组装中缺失的接线。

## Why

c278 完成后，`SessionStore` port 已上提 `fork`/`create`/`load_entries`/`build_session_context`，
但三条"接线"仍然断裂：

1. **`Command::SwitchSession` 是假实现**——rpc.rs 仅改 `s.session_id` 字符串字段，不 load、
   不 validate CWD、不重建会话上下文。比 c278 删除的零调用 `switch_session` 方法更弱。
2. **`Command::GetMessages` 是 stub**（"not yet implemented in RPC"）；`ExportJsonl` /
   `ImportJsonl` 在 `Command` 枚举中根本不存在，而 `AgentSession` 已有对应的活方法
   （`export_to_jsonl` / `import_from_jsonl`）。
3. **AGENTS.md / 系统提示 / skills 未接入 live prompt**——CLI 构造 `DefaultResourceLoader`
   却只取 `get_prompts()`（templates），`context_files` / `system_prompt` / `skills`
   全部丢弃；`AgentSession.prompt_opts` 仅用 `cwd` 初始化，系统提示硬编码 fallback
   `"You are a helpful AI assistant."`。这是 c278 清理 `build_system_prompt_from_loader`
   时确认的既有缺口（非回归）。

`process_prompt` / `BUILTIN_COMMANDS` 这套 slash 命令层是**完全断开的并行死系统**
（零调用方，`Handled` 分支无人消费）。本变更不复活它——RPC `Command` 通道才是真实的
命令路径，且已接线大半。

## What Changes

### P1 — 补齐 RPC 会话命令（interactive-protocol）

- **`SwitchSession` 真实化**：调 `store.exists` + `load_entries`（或新增 load_validated
  port 方法），校验目标会话存在后切换 `session_id` 并触发上下文重建。返回会话元信息。
- **`GetMessages` 实现**：调 `store.load_entries` 返回 `Vec<SessionEntry>`（经 protocol
  序列化）。移除 stub。
- **新增 `ExportJsonl`** + **`ImportJsonl`**：与既有 `ExportHtml` 对称，转发到
  `AgentSession::export_to_jsonl` / `import_from_jsonl`。
- 对应 `Command`/`Event` 枚举 + dispatch 补齐。

### P2 — 接入 AGENTS.md / 系统提示 / skills（agent-session）

- CLI 组合根：从已构造的 `DefaultResourceLoader` 取 `get_agents_files` / `get_system_prompt`
  / `get_append_system_prompt`，填入 `AgentSession.prompt_opts` 的 `context_files` /
  `system_prompt` / `append_system_prompt` 字段（当前全默认空）。
- 评估：skills 是否也经 `prompt_opts` 接入系统提示（当前 `SkillManager` 走
  `/skill:` 展开，与 system prompt 是两条路）。
- 删除硬编码 fallback `"You are a helpful AI assistant."`，改为 loader 未提供时用
  `build_system_prompt` 的内置默认。

### P3 — 决策：slash 命令层的去留（agent-session）

- 确认 `process_prompt` / `BUILTIN_COMMANDS` / `PromptResult::Handled` 确实零调用方后，
  **删除这套死系统**（commands.rs 的 builtin 表降级为 `GetCommands` 的返回数据源，
  `process_prompt` 的 slash 分支移除）。
- 保留 `!cmd` / `!!cmd`（bang）与 `/template:` / `/skill:` 路径——这些是活路径。

## Capabilities

- `agent-session`（modify）：prompt_opts 接入 resource loader；删除死 slash dispatch。
- `interactive-protocol`（modify）：SwitchSession 真实化、GetMessages 实现、Export/Import JSONL 补齐。

## Impact

- **用户可见**：RPC 客户端可正确 switch/export/import/list messages；CLI 实际使用
  AGENTS.md 与配置的系统提示（而非硬编码）。
- **代码**：rpc.rs dispatch + cli/mod.rs 组装根 + protocol.rs Command 枚举。
- **测试**：新增 BDD 覆盖 switch/export-jsonl/import/get-messages；snapshot 更新。
- **零回归**：既有 88 BDD + lib 测试不退化。
