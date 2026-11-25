# Tasks: c3345-refactor-ui-model-writes

## 1. 生产路径改走方法

- [ ] 1.1 按 `design.md` 加上 `mark_idle`。`recalc_busy` 的直接赋值分支、`end_gated_assemble_idle`、`lifecycle` 里清 Idle 的分支、`rebuild_scrollback_from_travel` 改为调用它。
- [ ] 1.2 bridge handler 里写 `Working` 的地方改为 `set_busy_status`。`recalc_busy` 里已经调用 `settle_after_turn_boundary` 的分支保持不变。
- [ ] 1.3 把 `history_window::apply_notice` 的插入、改写和删除挪进 `UiModel` 的一个方法。`history_window` 调用它。

## 2. 现有队列条与空闲场景

seam：现有队列条场景，以及 AgentEnd 回到空闲的 harness 切片。不新增 BDD。测试和 harness 里对字段的直接赋值不改。

- [ ] 2.1 这些场景保持通过。队列非空时，host 的空闲分支仍走 `mark_idle`，不改成 settle 的状态文字。

## 3. 临时评判（deslop-please）

不进 `just qa`。基线是 `.tmp/deslop-please.allow`。测试路径诊断见 `.tmp/deslop-please-deferred-tests.md`，本 change 不改。

- [ ] 3.1 在本 change 分支上跑 `deslop-please xylitol-app-tui`。`UiModel::phase` 和 `UiModel::status` 从报告里消失。`history_window::apply_notice` 不再出现在 `UiModel::history_notice` 的记录里。
- [ ] 3.2 `deslop-please xylitol-app-tui --allow .tmp/deslop-please.allow` 没有基线以外的新诊断。
