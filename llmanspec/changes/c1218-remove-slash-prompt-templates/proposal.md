---
depends_on: []
blocks:
- c1220-add-jinja2-prompt-templates
branch: sdd/c1218-remove-slash-prompt-templates
base_sha: eb765f4269cbb7d68bd3c46359ed2d5e552cb911
checkpointed: false
---

# 移除 slash prompt templates（pi `/name` / `/template:`）

> 探索结论（c1220 深挖 D2/D7/D8）：产品**明确不**支持 pi 式 user prompt templates；本 change 为 c1220 安全 minijinja 组装的**前置清理**。

## Why

slash prompt 模板路径在 xylitol 已半死：ResourceLoader 仍发现 `prompts/*.md` 并 `register_prompt_commands` 成 `template:{name}`，但生产展开（`process_prompt`）早已删除（c320）。同时 live 合约仍强制 pt3/pt4、a21/a22、a24 模板分发、rd* 列表 prompts 等——合约与产品意图脱节，并干扰后续 system prompt 重构。

## What Changes

- **删除**全局/项目 `prompts/*.md` 发现、`PromptTemplate` 运行时注册、`/template:` 命令面与 `$1`/`$@` 展开语义（含 BDD 辅助）。
- **改写/废止**相关合约：`agent-prompt` pt3/pt4；`agent-session` a21/a22 及 a24 中模板展开器条款、a25/a26 中 prompts 措辞；`runtime-resource-discovery` 中 prompts 列表/reload；`runtime-config` Settings.`prompts`（若仅服务该能力）；`test-standards` ts03 对 `templates.rs` 展开的要求。
- **保留**：skills / `$skill`、SYSTEM.md / APPEND_SYSTEM.md、AGENTS context、产品 slash 命令分发（a23 / a24 命令处理器侧）、chrome「不列 prompt templates」（atc18 可收紧为「无此能力」或保持 MUST NOT 列出）。
- **不**引入 Jinja；**不**改默认 `build_system_prompt` 语义（留给 c1220）。

## Capabilities

- `agent-prompt`（主）
- `agent-session`
- `runtime-resource-discovery`
- `runtime-config`（Settings.prompts，若确认仅服务 templates）
- `test-standards`（ts03 措辞）

## Impact

- 行为：用户不能再依赖 `/template:name` 或 pi `/review` 式模板；磁盘上遗留 `prompts/*.md` 被忽略。
- 合约面中等；实现多为删代码 + 改 BDD。
- 阻塞 c1220（见 `blocks`）；c1220 日后 rename 为 `c1220-add-safe-minijinja-system-prompt`。

## Out of scope

- 安全 minijinja / `render(ctx)`（c1220）
- skills 发现与 `$name` 注入
- SYSTEM.md 用户覆盖语义
