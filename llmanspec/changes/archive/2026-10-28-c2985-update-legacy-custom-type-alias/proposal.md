---
depends_on: []
title: 收敛 legacy custom-type 读别名
needs_specs_change: true
branch: sdd/c2985-update-legacy-custom-type-alias
base_branch: main
base_sha: 776968325c212ea230ab80e51c0b02d639968850
---

# 收敛 legacy custom-type 读别名

## Why

- spec `agent-session-store` 对 workflow / todo_checkpoint 写的是“读取 须兼容旧 custom_type 的一次性迁移路径，但新写入 不得再使用旧名”。
- 实现永久接受 `custom_type = "plan"`（workflow）与 `"plan_checkpoint"`（checkpoint），没有 rewrite-on-append，也没有一次性迁移；旧名会被无限期双解析。
- 提案（`c2944` 等）明确“不保留永久双解析别名”，当前实现与提案意图不一致。
- 对本地 pre-release 会话风险低，但属于 spec 漂移；未来若清理别名，旧会话会突然失去 workflow/checkpoint 状态。

## What Changes

- 明确迁移策略（见 design）：
  - **A. 一次性迁移**：会话加载/resume 时把旧 `plan` / `plan_checkpoint` 条目改写为新类型（append-only 追加新条目，读取只认新名）；
  - **B. 直接删除别名**：旧会话的 workflow/checkpoint 状态丢失（pre-release 可接受）；
  - **C. 保留永久别名**：修改 spec 文案，承认永久兼容。
- 若选 A，定义迁移触发点（加载 / resume / 首次读取）、幂等性与失败降级。
- 同步更新 spec 措辞与单测（旧名读取、迁移后新名读取、重复迁移不重复追加）。

## Capabilities

- `agent-session-store`：workflow / todo_checkpoint 的 custom_type 与迁移语义。
- `agent-workflow`：workflow 读取路径。
- `agent-todo`：checkpoint 读取路径。

## Impact

- `crates/xylitol-plugin-sdk/src/session/workflow.rs`：`LEGACY_CUSTOM_TYPE_PLAN`。
- `crates/xylitol-plugin-sdk/src/session/checkpoint.rs`：`LEGACY_CUSTOM_TYPE_PLAN_CHECKPOINT`。
- 会话加载 / resume 路径：迁移触发点。
- specs 与单测。

## Out of Scope

- 磁盘格式（v7）变更。
- `plan_mode` / `plan_doc` 的历史条目（它们没有旧别名）。

## Further Notes

- 由 PR #30 review F8 记录。
- 迁移是**写操作**：若在读取路径做迁移，需要确保只追加、幂等、失败不阻断读取。
