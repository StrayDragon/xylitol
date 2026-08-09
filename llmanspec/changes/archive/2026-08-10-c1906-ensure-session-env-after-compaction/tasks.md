# Tasks: c1906-ensure-session-env-after-compaction

> apply 完成；verify PASS；archived。

## 0. Review 门

- [x] 0.1 读 c1905 archive design D8 + 发现表
- [x] 0.2 与 c1897 边界确认（bootstrap vs 全栏）
- [x] 0.3 design 钉：组装时 ensure + 追加则 persist（三缝）

## 1. Specs landing（绑定分支）

- [x] 1.1 `agent-prompt` pt12：compact/overflow ensure 不变量 + scenario（`feature: false`）
- [x] 1.2 validate（change OK；BDD compaction 已随 post-compact env 校正）

## 2. 实现

- [x] 2.1 抽出 `ensure_session_env_in_history`
- [x] 2.2 ReAct 首轮注入改走 ensure
- [x] 2.3 overflow reload 后 ensure + persist
- [x] 2.4 `compact_session` 成功后 ensure + persist
- [x] 2.5 单测：cut / missing / cwd 变更

## 3. 校验

- [x] 3.1 触及单测 + 相关 BDD
- [x] 3.2 `just qa`（开 PR 前）
