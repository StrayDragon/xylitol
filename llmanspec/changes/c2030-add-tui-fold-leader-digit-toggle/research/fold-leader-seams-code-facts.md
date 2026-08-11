# Research: Fold Leader 代码接缝事实（c2030）

> 一手仓内代码；禁止推测「应有」API。对照草案见同目录 `fold-leader-vs-global-alt-e.md`。

## 1. Scrollback 条目身份与折叠轴

**真源类型**：`UiEntry`（`src/app/tui/bridge/model.rs` ~104–161），挂在 `UiModel.entries: Vec<UiEntry>`。**没有**跨变体的稳定 `EntryId` 类型。

| 变体 | 稳定键？ | 折叠服从 |
|---|---|---|
| `User` / `Assistant` / `ScrollNotice` / `Error` | 无 | 无 fold |
| `Thinking` | 无 id | **仅** `ScrollbackFold.thinking_expanded`（`scrollback.rs` ~620–632） |
| `Tool` | `id: String`（tool call id；`upsert_tool_entry`） | **`tools_expanded`** 块显隐；正文高度另受 **`tools_output_expanded`**（Ctrl+O） |
| `Diff` | 无 id | 同 Tool：`tools_expanded` + `tools_output_expanded` |
| `Ask` | `id: String` | 渲染用 **`tools_expanded`**（~782–801）；字段 `expanded: bool` 进 fingerprint 但 **从未写成 true**（grep 无赋值） |
| `Bash` | 无 id | **不**读 `tools_expanded`；命令行常显；输出只受 **`tools_output_expanded`**（~739–780） |
| `Compaction` | 无 id | **`compaction_expanded`**（Complete 态；~803–834） |

`ScrollbackFold`（`scrollback.rs` ~19–39）四全局 bool：`thinking_expanded` / `tools_expanded` / `tools_output_expanded` / `compaction_expanded`。默认：thinking 关、tools 开、viewport 预览、compaction 关。

会话树另有 **per-id** `folded_nodes: HashSet<String>`（`packages/xylitol-tui/.../tree_selector.rs`）——可借鉴模型，**不是** live scrollback。

## 2. Alt+E / Ctrl+T / Ctrl+O 路径

**目录**：`src/app/tui/keybindings.rs` `APP_KEYBINDINGS`：

- `app.thinking.toggle` → `ctrl+t`
- `app.tools.blocks` → `alt+e`
- `app.tools.expand` → `ctrl+o`

**处理**：`UiRoot::handle_slot_input`（`layout/root/slot_input.rs` ~228–247），在 `EditorSlot::Editor` 且其它槽已 return 之后、`editor.handle_input` **之前**：

```text
app.thinking.toggle  → fold.thinking_expanded ^=
app.tools.blocks     → fold.tools_expanded ^=  AND  fold.compaction_expanded ^=
app.tools.expand     → fold.tools_output_expanded ^=
```

三键各自独立 `if`；**共享收尾**：`scrollback_paint.invalidate()` + `bump_upper_gen()`。Alt+E **确实同时翻** tools 与 compaction（与旁注「同和弦」一致）。Ctrl+T / Ctrl+O **不**改 `tools_expanded` / `compaction_expanded`。

## 3. Paint-cache / fingerprint（ath25）

`ScrollbackPaintCache`（`scrollback.rs` ~454–489）：

- 槽：`entries: Vec<(u64, Vec<String>)>` 按 **entry 下标**对齐；`fold: ScrollbackFold`；`width`。
- `prepare(width, fold)`：若 `width` 或 **整个** `fold` 不等 → **清空全部** entry 缓存。
- `entry_fingerprint`（~491–567）：hash 变体判别 + 内容字段；**不含** fold / 外部 override。
- miss（~598–864）：fp 变 → `truncate(entry_idx)` 后从该点重画；`entry_misses++`。
- 全量 `invalidate()`：清 entries + streaming（fold 翻转今日走这条）。

ath25 护栏：`tests.rs` `scrollback_entry_cache_limits_misses_under_streaming`——流式尾部变更时 `entry_misses <= 2`（历史条目须 hit）。

**对 c2030 的硬含义**：单块 override 若仍改「整份 `ScrollbackFold`」并 `invalidate()` / `prepare` 清全表，会破坏 miss 上界。须让 override **进入该 entry 的 fingerprint（或旁路键）**，且 toggle **只**从目标 `entry_idx` 起 truncate——勿全历史 MD 重解析。

## 4. 视口可见性

引擎 `TUI` 持有私有 `previous_viewport_top`（`packages/xylitol-tui/src/tui.rs` ~287, ~1343）：content-end 对齐，`top = len.saturating_sub(height)`。**无**公开「列出视口内 scrollback 行 / 可折叠头」API。

产品侧：`UiRoot::render` 拼 loaded-resources + `render_scrollback` + queue + chrome + editor + footer（`layout/root/render.rs` ~191–223）；scrollback 是扁平 `Vec<String>`，**无** entry→行距映射表（`c1760` 提案才意向预留 `segment_id → [line_start, line_end)`）。

**结论**：c2030 须自建「从 scroll 偏移 + 已渲染行 / entry 行距」扫描可见 fold 头；今日无现成 API。

## 5. 数字键与 InputListener

Editor 聚焦时：`slot_input` 未匹配 app 折叠键 → `self.editor.handle_input(event)`（~252）。Editor 末尾 `printable_from_key_event` → `insert_ch`（`editor.rs` ~1558–1559）——**`0`–`9` 直接入草稿**。

预焦点监听：`install_ui_root_key_listeners`（`layout/root/mount.rs` ~82–104）经 `TUI::add_input_listener`；`dispatch_event` **先**跑 listener（`tui.rs` ~1000–1017），`Consumed` 则不到焦点组件。今日仅 `app.clear` / `app.interrupt`。

**可镜像**：FoldLeaderMode 用短命 InputListener（或 SharedUiRoot 上 listener）吞 digit/Esc；**不必**新开常驻 `EditorSlot`、不必永久抢焦点。退出后数字回 Editor。

## 6. c1760 和弦冲突（提案原文）

路径：`llmanspec/changes/c1760-add-tui-activity-fold/proposal.md`

- 已拍板表（~63–64）：`Alt+Shift+E` = `activity.expandNearest`；`Ctrl+Alt+Shift+E` = `activity.collapseNearest`。
- 与 Alt+E（~70）：分层；「L1 定点 leader 见并行 `c2030`（可能重载 Alt+E…须一并拍板）」。
- 深挖 C 表（~260–261）：同上动作 id / 默认和弦。
- 旁注风险（~226）：L2 行勿写 `(Alt+E)`，应标 `(Alt+Shift+E)`。

→ **勿**把 fold-leader 默默绑到 `Alt+Shift+E`；与全局 Alt+E 重载方案须和 c1760 文档对齐。

## 7. 最小数据模型与状态落点（据代码）

| 关切 | 证据导向建议 |
|---|---|
| Override 键 | Tool/Ask 已有 `id: String` → `HashMap<String, bool>`（或 enum 键）优先；Thinking/Diff/Bash/Compaction **无** UiEntry id → 首波仅 Tool(+Ask?)，或另造合成键（**须改类型**，今日无） |
| 全局默认 | 保留 `ScrollbackFold` 四 bool；`render`：`overrides.get(id).copied().unwrap_or(global)` |
| Fold / Leader 状态 | 今日 `fold` + `scrollback_paint` 在 **`UiRoot`**（`layout/root/mod.rs` ~64–68, ~151），**不在** `UiModel`。Leader 映射/超时同放 UiRoot；digit 吞取可挂 `mount.rs` InputListener + `SharedUiRoot` |
| Host | Host 已装 listener；Leader **不必**下沉 host 业务，除非要跨 layout 生命周期 |

---

**Open（代码未决，非本文件拍板）**：Bash 是否进编号平面（今日不受 Alt+E）；`Ask.expanded` 死字段是否复用；Alt+E 改 leader vs 另键（见 `fold-leader-vs-global-alt-e.md`）。
