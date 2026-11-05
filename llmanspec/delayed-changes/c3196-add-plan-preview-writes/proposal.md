---
depends_on: []
title: "Plan-mode preview writes"
---

## Why

Plan mode 目前是只读：write/edit 被拒绝，codemode catalog 也不含 edit。用户需要在计划阶段生成少量 preview HTML/UI 变体（例如一个组件多种 UX/UI，供用户选择下一步），这需要写入能力，但不应放开整个 repo。

## What Changes

- 新增受限的 plan preview 写能力：只允许写入指定 preview root，不写 repo 其他路径。
- 推荐形态：专用工具 plan_preview_write（path、content）；path 相对 preview root；扩展名 allowlist（html/css/js/svg/png 等）；大小/数量上限。
- plan mode 中间件 allowlist 加入 plan_preview_write；write/edit 仍拒绝。
- codemode（c3106，parked）在 plan mode catalog 中包含 plan_preview_write；不包含 write/edit。
- preview root 候选：项目内 .xylitol/plans/<plan-id>/previews/（trust 门禁）或临时目录；design 定。
- 生命周期：list/read/cleanup 可选；自动打开浏览器/本地 server 不在 MVP。
- 安全：path traversal/symlink/扩展名/大小限制；preview 内容视为不可信；不得自动执行。
- 测试：plan mode 允许 preview write、拒绝 repo write；路径逃逸；大小/数量；codemode 组合；trust。

## Options

- A 专用工具（推荐）：语义清晰，allowlist 简单，codemode 绑定明确。
- B write/edit 路径 allowlist：工具少，但容易绕过/难以审计。
- C 独立临时 workspace：更安全，但发现性和生命周期复杂。

## Capabilities

- agent-plan：plan mode preview writes。
- agent-tools：plan_preview_write。
- domain-security：path/trust/limits。
- agent-codemode（parked）：plan mode catalog 包含 preview 工具。
- app-tui（可选）：preview 文件打开/展示，后续。

## Impact

plan 插件、tools 插件、permission、codemode catalog、TUI（可选）。不改 provider/protocol。

## Open Questions

- preview root 位置与生命周期（项目内 vs 临时）。
- preview 是否持久/提交；cleanup 策略。
- 文件类型/大小/数量上限。
- 是否提供 preview server/打开浏览器；MVP 不做。
- 与 plan doc 审批、trust 的关系。
- codemode 在 plan mode 的 catalog 组合。

## Further Notes

- 场景：一个组件 3 个 UX 变体；写 previews/variant-a.html 等；用户 review 后选下一步。
- 本 change 是独立 draft；c3106 codemode 已 parked。
- 只 proposal；design/tasks 未开始。
