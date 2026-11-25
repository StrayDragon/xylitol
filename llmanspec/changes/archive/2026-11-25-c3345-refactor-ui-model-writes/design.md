# Design: c3345-refactor-ui-model-writes

## 目标

生产路径上，transcript 条目、历史窗口注解、`phase` 和 `status` 都经过 `UiModel` 的方法。

这里没有状态图。产品规格没有把「队列非空时必须显示 Delivering steering」写成可判定规则。

## 两种形状

| | 形状 A | 形状 B |
|---|---|---|
| 空闲 | 新增 `mark_idle`：`phase = Idle`，`status = None` | 所有离开 Busy 的路径都改调 `settle_after_turn_boundary` |
| 忙碌 | 已有的 `set_busy_status` / `set_busy_placeholder` | 同左 |
| 队列非空时的状态文字 | 保持调用点现在的文字。只有今天已经调用 settle 的地方继续调用它 | 队列非空时，host 的空闲分支也会写成 Delivering steering 或 Follow-up pending |

## 选择

用形状 A。

| 写入 | 谁调用 | 方法 |
|---|---|---|
| 会话事实条目 | bridge 的现有追加方法 | 保持这些方法为唯一追加点 |
| 历史窗口注解 | `history_window::apply_notice` | 挪进 `UiModel` 的一个方法，插入、改写或删除 index 0 的历史 `ScrollNotice` |
| 进入 Busy 并写状态文字 | host 的忙碌标签，以及 bridge handler 里写 `Working` 的地方 | `set_busy_status` |
| 直接回到 Idle 并清掉状态 | `recalc_busy` 里今天直接赋值的分支，`end_gated_assemble_idle`，`lifecycle` 里清 Idle 的分支，`session_tree::rebuild_scrollback_from_travel` | `mark_idle` |
| 队列条决定的收尾文字 | `recalc_busy` 里已经调用 settle 的分支 | `settle_after_turn_boundary` |

`layout/root` 的生产路径保持只读。测试和 harness 里对字段的直接赋值不在本 change 里改，见 `.tmp/deslop-please-deferred-tests.md`。字段可见性保持现状，避免为了消掉测试赋值去改那一批脚手架。

## 完成时的读法

现有队列条场景和 AgentEnd 回到空闲的 harness 切片保持通过。`deslop-please` 里，上面列出的生产路径不再作为 `UiModel::phase` 或 `UiModel::status` 的外部写入点出现。测试文件里剩下的写入留在基线里。
