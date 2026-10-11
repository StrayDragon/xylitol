# Tasks

测试缝：`UiRoot` 可见视口（经 AO 裁切或等价钩子，不单测未裁切 `render()[0]`）；滚轮再投影（`reproject_frame`）；`apply_switched_session` / 主机泵（`HostSession` + `tests/bdd/steps_app_tui_host.rs`）；harness 里 resume-render 切 session 断言。不新开 harness。不测真 TTY。

## T1 — 引擎视口叠层 + 产品栈改叠已裁切视口

- [x] `do_render`：AO `project_frame` / `reproject_frame` 之后（Inline 在 `render` 之后）做视口叠层。`composite_overlays` 从裁切前挪过来；AO 滚轮再投影也必须 composite。
- [x] `Component::overlay_viewport(&self, lines, width, transcript_rows)` 默认 no-op。`UiRoot` 实现：未过期 Copied / Copied Last Msg / Heuristic 叠在 `lines[0..transcript_rows]` 右上；`render()` 不再 `overlay_client_notices` 进内容缓冲。
- [x] 仍不计入 `last_dock_rows`；仍非 `Error: ` 通知条；不盖 dock。
- [x] 校验：内容高于视口且 `scroll_top > 0` 时，可见第一行含 `✓ Copied`；内容第 0 行不是落点。包测：reproject 后标记仍在可见顶（r1954）。

## T2 — [blocked-by: T1] 滚轮再投影仍钉住

- [x] 先 ingest（臂装 Copied），再只 `reproject_frame` 且 `scroll_top > 0`，不重绘 `UiRoot::render`。
- [x] 校验：可见顶行仍有 Copied。

## T3 — 导航瞬时改回滚动提示尾插

- [x] `apply_switched_session`（switched / new / cloned / imported / restored）改 `push_scroll_notice`；MUST NOT `push_toast_info_notice`。`forked →` / `history @` 保持尾插。
- [x] 改写 resume-render 单测 / harness：断言 entries 末尾有导航文案、通知栈不含；去掉「禁止 transcript 行」。
- [x] 校验：空闲切 session 后跟底可见 `switched →` 在输入框上方；栈里没有。

## T4 — [blocked-by: T1] `/history-copy-last` 成功进栈

- [x] 成功：通知栈 `✓ Copied Last Msg`；MUST NOT 尾插「Copied last assistant message」。无 assistant / 失败仍滚动提示，MUST NOT 臂装栈。
- [x] 改写 `harness_history_copy_last_*`：成功看栈文案，空列表仍看滚动提示。
- [x] 校验：有 assistant 时栈含 Copied Last Msg 且 entries 无 Copied last assistant；无 assistant 时不进栈。

## T5 — [blocked-by: T1, T3, T4] 门禁与 BDD

- [x] 改写 `client_notice_stack_right_aligns_copied_not_error_bar` 等到可见视口。
- [x] 绑定 r1930 `notice-stack-copied-pins-scrolled-viewport`、r1953 `session-switch-trails-not-stack`、r1193 成功进栈、r1954 `overlay-viewport-pins-after-reproject`；主机泵若直调未裁切 `render`，栈场景改为已裁切视口。
- [x] 校验：`cargo test --all-features --test bdd` 在本 change 分支相对 merge-base 绿（本票相关场景）。
