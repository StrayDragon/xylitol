# Design — TUI 编辑区 chrome 改版与详情下线

## 目标

把编辑区从「三 chip + 纯 dash」改成「圆角边框 + 右对齐阶段名 + 输入行右端 `◨` + 下边框折叠 chip + 条件重置 chip」；下掉详情 overlay；spinner 固定 `⬢` 颜色脉冲。保持固定区不增高、不换行、可点可达。

## 定稿视觉

```text
⬢ Drafting reply
╭────────────────────────────────────────────────────────── 新功能A ───╮
❯                                                                  ◨
╰──────────────────────────────────────────────────── [▤ 摘要]───╯
~/Projects/xylitol · local-model · high
```

- 圆角：`╭ ╮ ╰ ╯`。
- 阶段名：上边框右端，右对齐，不居中。
- `◨`：输入内容行右端，与 `❯` 同一行。
- 折叠 chip：下边框右端。
- 重置 chip：仅在有覆盖时出现，位于下边框折叠 chip 左侧；本帧无覆盖，不画。
- spinner：固定 `⬢`，颜色脉冲。

## 现状（代码事实）

- chip 列表：`footer_chip_actions`（`crates/xylitol-app-tui/src/layout/root/footer_chips.rs:40`）固定 `[详情] [侧栏] [重置折叠]`；`append_border_chips`（`:61`）绘制；`paint_editor_chips`（`layout/root/render.rs:10`）只替换第一行。
- 详情 overlay：`overlay_session_popup`（`layout/root/render.rs:129`）、`SESSION_POPUP_*` / `session_popup_open`（`layout/root/mod.rs`）、`SESSION_POPUP_HIT_ID`（`host/mod.rs:1032`）、`app.session.popup`（`keybindings.rs:120`）、`detailsCycle`（`:130`）。
- 侧栏：`app.session.sidebar` / `app.session.sidebarWidth`；`session_sidebar_on` / `session_sidebar_available`；标准档 ≥80（`layout/tier.rs`）。
- 重置 chip：`FooterAction::ResetFold` 常驻；`reset_chip_enabled` 只影响点击，不影响绘制；spec r3033 要求「存在非默认态或逐块覆盖时才额外绘制」。
- spinner：`Loader` 默认 braille frames（`crates/xylitol-tui/src/components/loader.rs:11`）；产品在 `theme_apply.rs:18` 与 `layout/root/mod.rs:151` 构造。
- DESIGN：chip 行 `DESIGN.md:166`；详情例外 `:201`；上边框 chip `:207`；窄档 alt+u `:220`。
- 阶段名由 c3115 落到 `UiRoot.stage_name`；折叠等级由 c3120 落到 `ActivityFoldSettings.display_level`。

## 详情下线

删除：

- `[详情]` chip / `FooterAction::Details`。
- `session_popup_open` / `session_popup_id` / `SESSION_POPUP_W` / `SESSION_POPUP_ROWS` / `SESSION_POPUP_HIT_ID`。
- `overlay_session_popup` 及其绘制 / 命中 / 收起路径。
- `app.session.popup` 与 `detailsCycle` 的 popup 分支；`detailsCycle` 无 popup 后冗余，删除动作与 r3032 条目。
- 相关 harness / snapshot / spec r2903 / r3031 的「会话详情弹层显隐」/ DESIGN 例外 / alt+u 降级描述。

保留：

- `app.session.sidebar` / `app.session.sidebarWidth`。
- 会话 id 查看：会话树 `app.session.toggleId`。
- 侧栏带 / 让宽 / 命中逻辑不变。

## 圆角

- 上边框：`╭` + dashes + title + dashes + `╮`。
- 下边框：`╰` + dashes + reset chip? + fold chip + dashes + `╯`。
- 内容宽不变；圆角只占边框两角。
- DESIGN「无圆角」改为「编辑区边框圆角」；r1224/r3033 文案同步。
- 窄档仍圆角；不因宽度降级成 plain。

## 阶段名落位

- 上边框右端：`╭──── <stage_name> ───╮`；title 前留 1 空格、后留 3 dashes。
- 右对齐，不居中；muted；不抢 accent。
- 预算：`width - 2 (corners) - title_w - 3 (trailing)` 给左侧 dashes；title 超宽 `truncate_to_width` + `…`；最小保留 2 dashes。
- 无阶段名：不留 gap，边框纯 dashes。
- 阶段名由 c3115 提供；本 change 只重排。

## 侧栏字形

- 字形：`◨`（U+25E8）；tmux 3.8 实测 1 列；备选 `▐` / `▥` / `⬒`。
- 位置：**输入内容行右端**，与 `❯` 同一行；不占边框行。
- 预留：glyph + 前导空格 = 2 列；编辑器文本宽度 = 原宽 - 2；输入文本在预留列前换行/截断；不得与 `◨` 重叠。
- 激活：侧栏开 → accent；关 → muted。
- 命中：2 列（glyph + 前导空格）；只靠指针可达；标准档 ≥80；窄档不画，编辑器恢复 2 列。
- 可访问性：无文字标签；保留 `app.session.sidebar` 动作 id 供 keybindings.json 重绑；`/sidebar` 可选（不做）。
- spec：r2904 / r3033 把 `[侧栏]` 改为 `◨` 内容行；r3031 保留侧栏指针入口。

## 折叠 chip

- 文案：`[▤ 摘要]`（用户确认 A）；显示当前等级；窄档隐藏。
- 位置：**下边框右端**；`╰──── [▤ 摘要]───╯`。
- 行为：左键循环 `app.activity.displayLevel`；激活态 accent；显示当前等级。
- 预算：标准档 ≥80；放不下整 chip 不画；不拆半。
- 命中：按屏上绘制列；`FooterChipHit` 同族；下边框命中需扩展 `paint_editor_chips` 到第二行。
- spec：r3033 新增折叠 chip；r3032 动作 id 已有（c3120）。

## 重置 chip

- 只在 `fold_has_overrides()` 或 `fold_differs_from_defaults()` 为真时绘制。
- 位置：下边框、折叠 chip 左侧；`╰──── [重置折叠] [▤ 摘要]───╯`。
- 文案 `[重置折叠]` 保持；点击等价 `app.tools.blocks`。
- 无覆盖时不占位；宽度不足先丢重置 chip，保留折叠 chip。
- spec r3033 已是此语义；本 change 修代码漂移。

## spinner

- 字形：固定 `⬢`；不换字形。
- 颜色脉冲：accent 亮 / accent 暗两档，100ms；例如 `\x1b[38;5;39m⬢\x1b[0m` ↔ `\x1b[38;5;32m⬢\x1b[0m`。
- 位置：状态行；与现有 `Loader` 同形；不增高。
- 实现：frames 预着色 + `spinner_color_fn` identity；若 `Loader` 对 frames 再做颜色包装，给 `LoaderIndicatorOptions` 加 per-frame color；不得让 ANSI 影响宽度（`visible_width` 已剥离）。
- 回退：若终端把 ambiguous 宽字符渲染成 2 列，固定 `●` + 颜色脉冲。
- 代码：抽 `spinner_frames()` 到 `layout/theme.rs` 或 `widgets/glyphs.rs`；两处 Loader 构造共用。
- spec：DESIGN spinner 字形段更新；r1224 不涉及。

## 布局预算

1. 圆角两角（各 1 列）。
2. 上边框阶段名：剩余宽度；超宽 `…`；最小 2 dashes。
3. 下边框折叠 chip：放不下整 chip 不画。
4. 下边框重置 chip：条件显示；放不下先丢。
5. 内容行 `◨`：2 列；窄档不画，文本恢复。
6. 所有元素同高；不得增高编辑区 / 另加行 / 换行。

## 命中

- `◨`：内容行最后 2 列；`app.session.sidebar`。
- 折叠 chip：下边框绘制列；`app.activity.displayLevel`。
- 重置 chip：下边框绘制列；`app.tools.blocks`（或 reset 语义）。
- 详情 chip：不存在。
- 所有命中以屏上实际绘制为准；未绘制不登记。

## 视觉预览（Python）

- 脚本：`llmanspec/changes/c3130-update-tui-editor-chrome/research/visual_preview.py`；说明 [`research/visual_preview.md`](./research/visual_preview.md)；彩色 HTML [`research/visual_preview.html`](./research/visual_preview.html)。
- 定稿变体：`rounded-pulse-summary`（B+E+D）。
- 其他变体保留用于对照：rounded/plain × hex/pulse/diamond × summary/quiet/recent/full。
- 运行：`python3.12 .../visual_preview.py --variant all`。

## Spec / DESIGN 变更点

- r1224：chip 行描述更新；圆角；阶段名右对齐；折叠 chip 下边框；`◨` 内容行；移除详情。
- r2903：整条删除。
- r2904：侧栏改 `◨` 内容行；窄档降级描述删除。
- r3031：移除「会话详情弹层显隐」；保留侧栏。
- r3032：移除 `app.session.popup` / `app.session.detailsCycle`；保留 sidebar / sidebarWidth；新增 displayLevel（c3120）。
- r3033：重写 dock chip 为「上边框阶段名 + 内容行 `◨` + 下边框折叠 chip + 条件重置」。
- DESIGN：chip 行、圆角、spinner、侧栏字形、详情例外删除、窄档 alt+u。

## 实施任务

1. 详情下线：删 chip / overlay / hit / pending / keybinding / tests / specs / DESIGN。
2. 圆角：上/下边框 `╭╮╰╯`；内容宽不变；snapshot。
3. 阶段名右对齐：title 段 + 预算 + 截断；依赖 c3115。
4. `◨` 内容行：预留 2 列；编辑器宽度调整；命中 2 列；激活/窄档；snapshot。
5. 折叠 chip 下边框：绘制 / 命中 / 循环等级；依赖 c3120；扩展 `paint_editor_chips` 到下边框。
6. 重置 chip 条件绘制：`fold_has_overrides` / `fold_differs_from_defaults`；下边框左侧；宽度不足先丢。
7. spinner：固定 `⬢` + 颜色脉冲；两处 Loader；回退；snapshot。
8. 测试：80/120/160 列 snapshot；无详情；标题截断；`◨` 内容行；折叠 chip 下边框；重置 chip 条件；spinner 帧；命中。
9. specs / DESIGN：上述条目；BDD binding。

## 验证

- snapshot：80 / 120 / 160 列；圆角；阶段名右对齐；`◨` 与输入同行；折叠 chip 下边框；重置 chip 条件；无详情 chip。
- harness：点击 `◨` 开/关侧栏；点击折叠 chip 循环等级；点击重置 chip 清覆盖；输入文本不与 `◨` 重叠。
- spinner：固定 `⬢`；颜色两档；100ms；宽度 1 列。
- 真机 tmux：120 列 / 80 列 / 窄档；侧栏；折叠 chip；圆角；颜色脉冲。

## 风险 / 待确认

- `◨` 在部分终端可能 ambiguous 宽 2 列；若预览发现错位，回退 `▐` 或 ASCII `|`。
- `◨` 占内容行 2 列会减少输入宽度；窄档隐藏；需 snapshot 验证长输入换行不重叠。
- 折叠 chip 下边框需要 `paint_editor_chips` 同时处理上/下边框；避免与 footer 命中冲突。
- spinner 颜色脉冲若 `Loader` 对 frames 做颜色包装，需加 per-frame color 或预着色 frames；不得让 ANSI 影响宽度。
- 重置 chip 位置（下边框折叠 chip 左侧）未在用户草图出现；条件显示时可接受，若用户要另放再调。
- 详情下线会失去「会话 id」overlay 入口；会话树 `app.session.toggleId` 保留，需在文档/帮助中提示。
- `detailsCycle` 删除是行为合约变更；r2904/r3032 需同步，避免悬空动作 id。
