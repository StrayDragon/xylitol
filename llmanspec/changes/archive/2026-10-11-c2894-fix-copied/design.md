# 设计：通知栈钉视口；导航瞬时回尾插

## 两槽（查漏后）

| 槽 | 谁用 | 本票 |
|---|---|---|
| **通知栈**（右上、TTL、不进 transcript） | Copied ✓、**Copied Last Msg** ✓、Heuristic ◆ | 改叠**已裁切视口**；滚轮再投影仍钉 |
| **滚动提示**（尾插、跟底在输入框上方、不 TTL） | `history @`、`forked →`、**以及改回的** `switched →` / `cloned →` / `imported →` / `restored →` / `new session →` | 从 `push_toast_info_notice` 改回 `push_scroll_notice` |
| **通知条**（`Error: `、TTL、输入区上方） | busy 拒切、reload 软闸、LINK_DOWN / RECOVERED | **不动** |

`push_toast_info_notice` 在导航迁走后只服务 Heuristic（及测试夹具）。不要为切 session 再发明第四槽。

resume-render 把导航放进栈，是为了「rebuild 不要把 tip 卡在 transcript 中间」。尾插发生在 **rebuild 之后**、且不写入 SessionEntry，下次 rebuild 自然丢掉旧 tip，不会卡在历史中间。跟 `history @` / `forked →` 同一模型。

## Copied / 栈：视口叠层

现状：`overlay_client_notices` 写进内容第 0 行，再 `project_frame`；滚轮 `reproject_frame` 不 render。`composite_overlays` 同样在裁切前，TopRight 也会滚走。

选定（`research/viewport-overlay.md`）：与选区/悬停同类，**裁切之后**做视口叠层。

```text
ingest:  render → project_frame → 视口叠层
滚轮:    reproject_frame        → 视口叠层
Inline:  render                 → 视口叠层
```

1. `composite_overlays` 挪到视口叠层（屏格锚点；AO 滚轮也跑）。
2. `Component::overlay_viewport(&self, lines, width, transcript_rows)` 默认 no-op。`UiRoot` 把 Copied / Copied Last Msg / Heuristic 画在 `lines[0..transcript_rows]` 右上，不盖 dock。`render()` 不再往内容缓冲叠栈。

不要按 `scroll_top` 改内容行。不要为装饰拉 `ao_components_stale`。不要把 Copied 文案写进 xylitol-tui（r1640 库只发信号）。不改 content-end / follow-bottom。

## 导航瞬时

`apply_switched_session` 末尾改为 `push_scroll_notice(note)`，与 `apply_session_tree_fork` / `apply_session_tree_travel` 对齐。改写 harness/tests 里 resume-render「必须在栈、禁止 transcript 行」的断言。

## 已核对、不改

- busy 拒切 / reload 软闸 / LINK_* → 通知条（E）
- slash 失败、export、compact、trust 报告 → 已是滚动提示（C/B）
- 换模成功 → 页脚（D），不刷滚动提示
- `/history-copy-last` 成功：`effects/slash/misc.rs` 改臂装通知栈 `Copied Last Msg`（✓），与鼠标 `Copied` 分文案、同槽。无 assistant / 失败仍 `push_scroll_notice`。改写 `harness_history_copy_last_*` 对 ScrollNotice 含 chars 的断言。

## 测试

| 缝 | 覆盖 |
|---|---|
| `UiRoot` / AO 投影 | 内容高于视口 + `scroll_top > 0`：可见第一行 `✓ Copied` |
| `reproject_frame` + r1954 | ingest 后再只投影：Copied / 标记仍在可见顶，内容第 0 行没有 |
| 主机泵 / harness | 空闲切 session：`entries` 末尾 `switched →`，`client_notices` 不含；跟底帧在输入区上方能看见 |
| 既有假绿 | `client_notice_stack_right_aligns_copied_not_error_bar` 改看可见视口 |
