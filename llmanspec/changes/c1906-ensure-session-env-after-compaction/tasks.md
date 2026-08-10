# Tasks: c1906-ensure-session-env-after-compaction

> 规划壳；Branch binding / Specs landing / apply 在 **c1905 归档后**（或本依赖已落地）再开。

## 0. Review 门

- [ ] 0.1 读 [`c1905/design.md`](../c1905-update-system-prompt-stable-volatile-split/design.md) D8 + 发现表
- [ ] 0.2 与 [`c1897`](../c1897-update-compaction-status-bar-messages/proposal.md) 边界确认（bootstrap vs 全栏）
- [ ] 0.3 design 钉：persist 时机（compact 完成 vs 仅组装时 ensure）

## 1. Specs landing（绑定分支）

- [ ] 1.1 `agent-prompt` 或 `agent-runtime`：compact/overflow 后 session_env 不变量（单测场景，`feature: false` 可）
- [ ] 1.2 validate

## 2. 实现

- [ ] 2.1 抽出 `ensure_session_env_in_history`（复用 `should_append_session_env` / `snapshot_for_cwd`）
- [ ] 2.2 ReAct 首轮注入改走 ensure（行为不变）
- [ ] 2.3 overflow reload 后调用 ensure（必要时 persist）
- [ ] 2.4 （可选）`compact_session` 成功后 leaf 无有效 env 则 persist 一条
- [ ] 2.5 单测：cut 掉早期 env → ensure；多条取最新 + cwd 变更校正

## 3. 校验

- [ ] 3.1 触及单测 + 相关 lint
- [ ] 3.2 `just qa`（开 PR 前）
