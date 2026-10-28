# Design

## Evidence

- `llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature:265-285`（`r2936`）：窗口上限 800；最旧一页回落为提示行 `↑ 加载更早的 200 条（还有 N 条）`；盘上无更早条目时不得渲染；短会话不得出现分页提示。
- `crates/xylitol-app-tui/src/history_window.rs:32-34`：`load_earlier_notice_text()` 只返回 `↑ 加载更早的 200 条`。
- `crates/xylitol-app-tui/src/history_window.rs:36-63`：`want_notice = has_earlier || drop > 0`；裁剪 `drain(..start)` 丢掉最旧一页。
- `crates/xylitol-app-tui/src/host/session_ops.rs:622-626`：`oldest_loaded_entry_id()` 返回 `LoadedHistory.first_entry_id`（最早已加载，不是最早已显示）。
- `crates/xylitol-app-tui/src/history_window.rs:8-24`：`LoadedHistory` 保留所有已加载条目。
- `crates/xylitol-plugin-sdk/src/ports/session.rs:40-48`：`load_entries_page` 只返回 `(entries, has_more)`，没有剩余条数。

## Failure modes

1. **文案缺计数**：`↑ 加载更早的 200 条` 与 spec 的 `（还有 N 条）` 不一致。
2. **溢出误报**：`drop > 0` 但 `has_earlier == false` 时仍显示“加载更早”，点击后回源为空 → 提示消失。
3. **被裁条目不可达**：裁剪后游标仍是“最早已加载”，比所有已加载条目都旧；回源返回空，被裁掉的 200 条永久丢失。
4. **内存未受限**：`LoadedHistory` 保留全部已加载条目，窗口上限只约束渲染。

## Options

### Option A：两种提示 + 显示游标 + 有界缓冲（推荐）

- 维护两个游标：
  - `oldest_loaded_entry_id`：已加载的最旧条目（用于回源判断）；
  - `oldest_displayed_entry_id`：当前渲染区最旧条目（用于“展开被裁内容”）。
- 提示分两种：
  - 盘上还有更早：`↑ 加载更早的 200 条（还有 …）`；
  - 仅窗口溢出：`↑ 更早内容已收起（点击展开）`。
- 点击溢出提示：优先从 `LoadedHistory` 缓冲恢复被裁页；缓冲不足时以 `oldest_displayed_entry_id` 回源。
- 缓冲上限：窗口 + 一页（或按决策保留全部已加载，但需显式说明内存边界）。

优点：语义清晰、可达性修复、内存可解释。
缺点：需要新增游标与两种提示分支；文案里的“N”仍取决于存储 API。

### Option B：扩展分页 API 返回剩余条数

- `load_entries_page` 返回 `(entries, has_more, remaining)`；存储层用 segment 元数据计算。
- 优点：可精确显示 `（还有 N 条）`。
- 缺点：SDK trait 变更（破坏实现方，需要 `SDK_API_VERSION` bump）；存储层要保证计数准确（sealed/active segment、anchor 位置）。

### Option C：保留单一提示 + 全量缓冲

- 只修文案与游标；`LoadedHistory` 保留全部条目，溢出提示点击从缓冲恢复。
- 优点：改动最小。
- 缺点：内存仍随加载页增长；与“窗口上限 800”的意图不符。

## Recommendation

采用 **Option A**；文案计数按以下优先级：

1. 若存储层能低成本给出剩余条数（segment 元数据已有计数），走 Option B 的只读扩展；
2. 否则把 spec 文案改为 `↑ 加载更早的 200 条（还有更早内容）`，避免伪造精确数字。

> spec 当前写的是“形如”，但仍以 `N` 示例；需要确认是“必须精确计数”还是“允许定性描述”。

## Spec wording (draft)

`r2936` 追加/改写：

> 窗口溢出且盘上无更早条目时，提示行 须表达“更早内容已收起”，不得复用“加载更早”；点击后 须恢复被裁页并保持滚动锚点。盘上仍有更早条目时，提示行 须同时表达本页条数与剩余量（精确条数或“还有更早内容”，按实现能力二选一）。

## Test seam

- BDD：构造 >800 条会话，加载完所有分页 → 触发裁剪 → 断言提示文案、点击后条目恢复、锚点不跳。
- 单测：`trim_to_window` 在 `has_earlier=false && drop>0` 时不得产生“加载更早”文案；`oldest_displayed_entry_id` 游标正确。
- 短会话：不得出现任何提示（已有覆盖，回归确认）。

## Risks

- 回源以 `oldest_displayed_entry_id` 为锚点时，存储层需支持“中间位置向前翻页”；当前 `locate_anchor` 已支持任意 anchor，但需要测试覆盖裁剪后的锚点。
- 缓冲恢复与回源可能重复插入条目；需要去重（按 entry id）。
- 若选 Option B，SDK trait 变更会影响第三方 store 实现；需要迁移说明。

## Open questions

1. 提示里的剩余量：精确 `N`（扩展 API）还是定性“还有更早内容”（改 spec）？
2. `LoadedHistory` 的内存边界：窗口 + 一页，还是保留全部已加载？
3. 溢出提示的点击语义：展开缓冲，还是回源重新分页？

## Decision (implemented)

- 采用 Option A：提示分「加载更早」与「更早内容已收起」，剩余量用定性文案。
- `LoadedHistory` 在切换/恢复/旅行/fork/debug 场景初始化；`merge_older` 按 entry id 去重。
- 盘上仍有更早条目时回源加载；否则从已加载缓冲展开被裁页；两者保持滚动锚点。
- `app-tui-fixed-zone` 的 `r2936` 与新增「窗口溢出后提示为收起而非加载更早」场景已落地。
