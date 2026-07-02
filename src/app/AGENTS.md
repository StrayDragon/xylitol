# src/app/ 应用面

本文件只给入口，不重复架构事实。分层不变量、应用面状态表、seam（`composition → Driver → XyEvent`）的 SSOT 是 `src/AGENTS.md`；全局规则见根 `AGENTS.md`。

- 代码架构 / 分层 / 应用面状态：`src/AGENTS.md`。
- 新增或改造应用面（interactive REPL / TUI / server 客户端 / 未来 GUI）的方法论：`write-surface` skill（`.agents/skills/write-surface/SKILL.md`）。
- 新增应用面前必跑的死代码分诊：`audit-dead-code` skill（`.agents/skills/audit-dead-code/SKILL.md`）。
- TUI 子目录细则：`src/app/tui/AGENTS.md`。
