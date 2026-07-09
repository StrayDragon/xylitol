# 提示词：修剪 write-surface / audit-dead-code 过时描述（用后即弃）

> 快速 agent 执行；主线审核。

## 问题

`.claude/skills/write-surface/SKILL.md` 与 `audit-dead-code/SKILL.md` 含易腐数字/过时指针，例如：

- 「当前仓库 31 处 `#[allow(dead_code)]`」——会过期
- write-surface「先读根 AGENTS.md 的项目地图/分层不变量」——分层 SSOT 已在 `src/AGENTS.md`
- 对 TUI 仍可能暗示旧 ratatui / 旧目录布局

## 任务

1. 读两份 SKILL + `src/AGENTS.md` + `src/app/tui/AGENTS.md` + `write-tui` skill。
2. 改写为**稳定规则**：
   - 保留：死代码三类分诊、复用契约、禁止 skeleton spray、TUI 走 xylitol-tui host 驱动。
   - 删除：具体「N 处 allow」；改为「先 `rg allow(dead_code)` 再分诊」。
   - 指针改为 `src/AGENTS.md` / `write-tui` / `src/app/tui/AGENTS.md`。
   - 明确：产品 TUI 以 `packages/xylitol-tui` 为准，禁止恢复 in-tree ratatui 引擎。
3. **不要**把进度表、change 列表写进 skill。
4. 若 `write-surface` 步骤 1「新增应用面必跑 audit」对**纯合约变更（如 c450）**过严，加一句例外：仅改 llmanspec/文档时可跳过；**第一个写 `src/app/tui` 代码的 change（c460）必须跑**。

## 完成标准

- [ ] 两份 SKILL 无过期计数
- [ ] 与现行 AGENTS 不矛盾
- [ ] 短 diff，可单独 chore 提交
