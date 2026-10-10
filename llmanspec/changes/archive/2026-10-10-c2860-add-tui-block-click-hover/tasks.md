# Tasks

测试缝：`src/app/tui/tests.rs` 合成 Mouse（无真 TTY）、`tests/bdd` 既有 transcript/host 折叠场景、产品 host `dispatch_event`。不新开 harness。

## T1 — 产品命中改走 mark_hit

- [x] host 摘掉 `install_fold_triangle_hit_priority`；ApplicationOwned begin 后 `set_pointer_click_phase(UpIfEmptySelection)`、武装探针。
- [x] paint 同步 `replace_hits`；`take_pointer_click` → 既有 `FoldTarget` toggle。id 仍只在产品。
- [x] Compaction 只登记三角列；summary 正文 MUST NOT 进可点体。待办栏不改。
- [x] 校验：既有「点三角列 toggle」harness 仍绿；点 Compaction summary 不翻全局态。

## T2 — [blocked-by: T1] 叶块空选点正文

- [x] Tool / Diff / Ask / Thinking：收起块头全宽可点；展开后空选 Up 点 painted 正文 toggle；拖或有选区不 toggle、不 OSC52。body 不 capturing。
- [x] z-order：内层后登记；att30 hint 带只折输出视口。
- [x] 校验：harness 点块头展开、空选点正文折回、划词不 toggle；点 `ctrl+o to fold` 只折视口。

## T3 — [blocked-by: T1] ActivityFold 仅块头

- [x] 信封 / 簇只登记块头行（含折叠标记，全宽）。展开后的嵌套块与夹心助手正文不登记为该级命中。
- [x] Asking questions 整行可点例外保持 att31/att33。
- [x] 校验：点簇/信封块头定点 toggle；点夹心助手正文不折信封；点内层工具只动该工具。

## T4 — [blocked-by: T1] 悬停 dim 与停丢 Moved

- [x] fan-in：探针武装时 Moved MUST 进 `dispatch_event`；未武装仍可丢。同 hover id 不整帧。
- [x] 悬停 dim 块头 / 可见左边轨 / 提示带，不 dim 输出正文；拖选中不画。
- [x] 校验：Moved 落在块头请求廉价重绘且组件树非整段 stale；拖选中无悬停 dim。

## T5 — [blocked-by: T2, T3, T4] designing 晋级

- [x] `git mv designing/tui-lab/modules/hover-highlight` → `designing/tui/modules/hover-highlight`；`surface: tui`。
- [x] intent：dim + 块头/轨/提示带；禁止整块淡底 / 满屏闪。固定态覆盖收起块头、展开指针在正文（亮轨）、Activity 块头、hint 带。
- [x] `just gen-designing-index`；改了产品帧则 `just export-design-frame`。
- [x] 校验：`just check-tui-designing` / `check-tui-tokens` 绿。

## T6 — [blocked-by: T2, T3, T4] BDD 对齐已落地 specs

- [x] 绑定新场景步骤（叶块块头全宽、空选点正文、簇头摘要列、悬停廉价重绘）；既有三角单击步骤仍绿。r1359 不改句。
- [x] 场景名与步骤仍绑 `tests/bdd`。
- [x] 校验：相关 `cargo test --all-features --test bdd` 与 `src/app/tui/tests.rs` 折叠/鼠标用例绿。

不在本任务列 finalize。
