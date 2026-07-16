# Design — c1170-refactor-app-tui-god-modules

## 目标形状（落地）

```text
host/
  mod.rs            — HostSession 字段 + step 调度 + 薄转发
  pending.rs        — PendingOps（已有）
  input_policy.rs   — stale Esc / busy / idle Enter / Ctrl+G
  session_ops.rs    — mount_* / apply_* / switched session

layout/
  slash_catalog.rs  — product_slash_commands
  root/
    mod.rs          — UiRoot 字段 + 薄 API + Component 转发
    slot_input.rs   — handle_slot_input 槽路由
    slot_nav.rs     — Esc / open/close slot
    render.rs       — 各槽 render_* helpers
  slots.rs / theme / session_tree — 已有

effects/            — 由 effects.rs 升为目录
  mod.rs            — drain_pending 编排 + refresh_footer_tokens
  slash.rs          — PendingSlash 臂（仍仅经 drain_pending）
  pending_ui.rs     — tree / models / import / resume pending
  bang.rs           — run_interactive_bang
  helpers.rs        — deepest_tree_id / switch_and_rebuild / stats dump

bridge/
  mod.rs            — re-export + apply_xy_event + 小 helpers + tests
  model.rs          — UiPhase / QueueBadge / UiEntry / UiModel + methods
  handlers/         — 已有族
  session_tree.rs   — 已有
```

## 硬约束（不可违反）

1. **语义不变**：abort 抑制时序、bang Esc、队列键位、ath6 单一 drain 泵不变。
2. **无 reach**：layout/widgets 仍 MUST NOT 调 Driver。
3. **公开路径稳定**：`crate::app::tui::{effects::drain_pending, bridge::UiModel, …}` 对外符号保持可编译（模块路径可经 `mod.rs` re-export）。
4. **行数目标**：拆后 `host/mod.rs`、`layout/root/mod.rs`、`effects/mod.rs`、`bridge/mod.rs` 各显著低于约 800 行；任何单文件逼近约 1200 仍视为硬味。

## 明确不做

- 合并 Host/UiRoot 双 UiModel
- 改 slash 名称 / SSOT（→ c1175）
- 解冻 Plate/Settings/Choice
