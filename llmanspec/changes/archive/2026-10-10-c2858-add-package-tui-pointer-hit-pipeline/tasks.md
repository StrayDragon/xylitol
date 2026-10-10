# Tasks

测试缝：`TUI::dispatch_event(InputEvent::Mouse)`、命中表 / click 相位 / 探针的包 API、`SelectionController` 选区与 OSC52。复用 `packages/xylitol-tui/tests/interaction_modes_test.rs` 与 `tests/bdd/steps_package_tui_interaction_modes.rs`。不新开 harness。

## T1 — HitRegionList 与 live scroll 映射

- [x] 包 API：content 坐标命中区 `{id, row, col_start, col_end, height}`；screen→id 在事件时刻读 `ScrollView` 的 scroll_top。
- [x] 空表 ≡ 今日无 hook。id 不承载产品折叠种类。
- [x] `HitPriorityFn` 可继续登记；内部适配为「该格有 id」。
- [x] 校验：包单测 — 滚动前后同一屏幕格映射到不同 content 行；`hit_priority_runs_only_on_left_down_not_drag` 仍绿。

## T2 — [blocked-by: T1] click 相位

- [x] 默认 `DownSwallow`：Left Down 命中则 swallow、不启拖；Drag/Up 不报 click。
- [x] opt-in `UpIfEmptySelection`：仅当无选区且未拖时 Up 报 click；空 click MUST NOT OSC52。
- [x] 校验：默认相位包测与现 ath33 口径一致；opt-in 有独立单测。

## T3 — [blocked-by: T1] 指针探针与廉价绘出

- [x] 未武装：Moved 仍不强制整帧（r1632）。
- [x] 武装后 Moved 送达；同一 id 不排帧；id 变 → reproject + 可见行 overlay，MUST NOT `ao_components_stale`。
- [x] hover 层独立于 selection invert。
- [x] 校验：`interaction_modes_test` 断言 InputReaction / 非 stale；render 计数不随同 id Moved 增长。

## T4 — [blocked-by: T1] capture 先于选区

- [x] capturing / overlay 命中在 transcript 拖选之前。
- [x] Down 闩锁直到 Up；期间 Drag 不启 transcript 选区。
- [x] 校验：包单测点 capturing 区并拖，选区仍空。

## T5 — [blocked-by: T2, T3, T4] BDD 场景绑定

- [x] `package-tui-pointer` 场景绑到 `tests/bdd/bindings_package_tui_pointer.rs`；步骤扩既有 `tests/bdd/steps_package_tui_interaction_modes.rs`（同一 `dispatch_event` 缝）。
- [x] 校验：`cargo test --all-features --test bdd` 在本 change 分支相对 merge-base 绿（仅本 capability 相关场景）。

## T6 — [blocked-by: T5] 包门禁

- [x] `just test-tui`（或 `cargo test -p xylitol-tui`）绿；相关 clippy。
- [x] 若公开 API 变化：`packages/xylitol-tui/AGENTS.md` host checklist 补一行指针（不把 FoldTarget 写进包）。

## T7 — host 口语 API

- [x] `Hit::on_line`（正文随滚动）/ `Hit::at_cell`（钉可见格）+ `TUI::mark_hit` / `replace_hits`；`HitRegion` 留作引擎存储。
- [x] 校验：包测 viewport 滚动后同一屏幕格仍命中；BDD `viewport-hit-stays-on-screen-cell`。

不在本任务列 finalize。
