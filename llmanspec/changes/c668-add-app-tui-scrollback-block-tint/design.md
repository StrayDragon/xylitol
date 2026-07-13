# Design — c668-add-app-tui-scrollback-block-tint

## Decisions

| 主题 | 决议 |
|---|---|
| 块模型 | 新增 `UiEntry::Bash { command, status, output, exclude_from_context }`（名称可微调）；status ∈ pending / success / error / cancelled |
| Tint | 复用 `tool-pending-bg` / `tool-success-bg` / `tool-error-bg`；cancelled → error tint + 文案 `(cancelled)` |
| 空行 | 与 agent_demo 一致：每块 **前+后** 各一行空白；相邻块之间视觉为双空行 |
| Agent vs bang abort | agent → System `Aborted`；bang → 块内 `(cancelled)` |
| bash-mode.md | 删除「bash 仅用 fg、非 tool bg」；改为块 tint MUST |

## Non-goals

- chunk 流式上行 / host 三路合流 → **c669**
- 新 Palette token（除非 DESIGN 先增）
