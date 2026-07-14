# Design — c715-update-app-tui-qa-baseline

## 分期位置

```text
c715 S0 护栏（本变更）──► c720 abort 竞态 ──► c725 共享 bang 环 ──► c730 host 拆分
         │
         └─ 并行文档：c716 AGENTS 布局地图（不改代码行为）
```

刻意差异不得回退：`src/app/tui/PI_DELTAS.md` A01/A02；包层 D09。

## Esc / busy / overlay（护栏 SSOT 摘要）

| 场景 | Esc 归属 | 结果 |
|---|---|---|
| Idle 空 editor | `UiRoot::on_escape` | 双 Esc → 开树 |
| Idle + `suppress_idle_esc` | `try_suppress_stale_esc` | 吃 Esc，不开树 |
| Busy 无 overlay | `try_busy_input` | `pending_abort` |
| Busy + overlay | `on_escape` | 关槽，不 abort |
| Agent abort 消费 | `drain_pending` → `note_user_abort` | Aborted + `suppress_xy` |
| Bang abort 消费 | bang 环 → `note_bash_cancelled` | `(cancelled)`，无 `suppress_xy` |

**禁止**在本变更把 bang/agent 两条 note 路径糊成一条。

## Harness 标签（BASE 闸）

| 标签 | 含义 | 本变更动作 |
|---|---|---|
| BASE | 全程必绿（H1–H27、B1–B7、c630/c650/c999、树测…） | 保持绿 |
| ABS | abort/suppress（h7、c665、c670…） | 保持绿；竞态补测可加但语义不变 |
| HRS | 手写 bang Esc `select!` | **折叠进共享 helper** |
| RPB | 走 `run_pending_bash`（无 Esc） | Esc 敏感路径改走 helper；纯完成态可保留 await |

外圈：`tests.rs` busy-Esc-not-tree；`bridge` abort dedupe；PTY `pty_product_fake_bang_esc_*`。

## bang helper 边界（相对 c725）

| 本变更 (c715) | 下一变更 (c725) |
|---|---|
| 抽出 helper；harness + 生产**都调用**它 | 删除残余分叉；落实 ath7「单一扇入」字面 |
| 允许 `mod.rs` 仍 `if take_bash { helper(...) }` | 与主环进一步合一 / 去嵌套味 |
| **不**改 abort 抑制时序 | c720 先修 suppress 竞态 |

## BDD 落点

| Feature（建议名） | 覆盖 ati30 场景 |
|---|---|
| `tests/features/app-tui-abort.feature` | abort-and-late-delta |
| `tests/features/app-tui-bang.feature` | bang-esc-and-second |
| `tests/features/app-tui-esc-overlay.feature` | esc-overlay-vs-tree |
| `tests/features/app-tui-queue.feature` | queue-keys |

Step 实现：优先 `tests/bdd_tui.rs`（或 `bdd.rs` 子模块）+ 公开/crate 测试钩子访问 `HostSession` 泵；**MUST** 经 `drain_pending` / 共享 bang helper，禁止复制 `effects` match。

语言：`# language: zh-CN`，与现有 feature 一致。

## rstest-bdd 升级

- 现状：`0.6.0-beta2` → 目标：`0.6.0-beta3`
- 先升依赖再跑全量 bdd；若 API 微变，只改 step/宏，不改场景语义
- `test-bdd` valid_scope 若新增 `tests/bdd_tui.rs`，归档合并时扩主 spec scope（apply 时改主 spec 或本 delta 注明）

## 权衡

- **为何不直接做 c725**：无同构测先合并环，回归不可定位。
- **为何 BDD + harness 双轨**：harness 细粒度/驱动计数；BDD 固定产品叙事，便于评审与防静默行为漂移。
- **demo vs 产品**：不把 demo Esc 归属「对齐」进产品；产品 host 先于 dispatch 的分层保持。
