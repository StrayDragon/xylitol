---
depends_on: []
needs_specs_change: true
branch: sdd/2026-10-review-fixes
base_branch: main
base_sha: c10be4e9983564fac14b797ff9c6defaff15c4cf
---

# attach 生效模型同步改 transient 下行（journal 去噪）

## Why

c2841 的 `sync_model_downlink`（`src/app/server/host.rs`）在**每次** `slot()` 绑定后调用，经 `append_and_push` 落 journal 并消耗 seq。内容幂等（同一写者当前模型），体积不幂等：TUI 重连/多端 attach 频繁时 journal 持续膨胀、replay 消费者收到重复 ModelSelect。对照同文件 `push_resources` 明确「Not journaled (must not consume seq)」——attach 同步是会话收敛事件而非会话历史，不应进 journal。

会话历史的模型变更已由领域层 `SessionEntry::ModelChange` 承载（`src/protocol/session/`），journal 层的 ModelSelect 只是下行收敛信号——职责应分离。

## What Changes

1. `server-core` 追加规则 **r1926「attach 生效模型同步不进 journal」**：绑定期 ModelSelect 同步 MUST 为 transient 推送（不消耗 seq、不落 journal）；重复绑定产生的重复下行 MUST 语义幂等；历史记录职责归领域层 ModelChange 条目。
2. `SessionSlot::sync_model_downlink` 由 `append_and_push` 改为与 `push_resources` 同族的 transient 推送（`downlink_server_request` 直推，不进 journal）。
3. BDD 场景（apply 落地）：同一会话二次 attach → journal max_seq 不变 + 订阅者仍收到 ModelSelect 下行；锚点由路径改指步骤/回归 fn。

## Capabilities

- `server-core`（会话槽绑定与下行分发）

## Impact / 风险

- replay 语义变化：journal replay 不再含 attach 期 ModelSelect——但每次 bind 后紧跟 transient 同步，徽标收敛路径不变（remote driver `resolve_model_info` 已兼容事件晚到）。
- 单订阅者窗口：bind 时序为 replay → transient 同步，无丢失窗口。
