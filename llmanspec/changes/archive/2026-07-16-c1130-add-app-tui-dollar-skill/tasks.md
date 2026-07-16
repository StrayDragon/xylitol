# Tasks — c1130-add-app-tui-dollar-skill

## 1. Expand（agent）

- [x] 1.1 `skill_expand`：parse `$name`、读 SKILL.md、拼 `<skill>` 块
- [x] 1.2 `run_react_loop`：发模型前对 user LlmMessage 展开；历史保持 raw
- [x] 1.3 单测：多引用 / 未知透传 / 正文注入

## 2. TUI

- [x] 2.1 `DollarSkillSource` + catalog from `loaded_skills`
- [x] 2.2 scrollback 用户行 skill-ref 高亮
- [x] 2.3 轻量 harness（非主门禁）

## 3. 校验

- [x] 3.1 `LLMANSPEC_BASE_REF=main llman sdd validate c1130-… --no-interactive`
- [x] 3.2 `just qa`（或 lint + 相关测）
