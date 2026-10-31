---
depends_on: []
title: 鼠标优先交互：键位收缩与屏上可直接操作
---

# 鼠标优先交互：键位收缩与屏上可直接操作

## Why

产品 TUI 主路径靠 9 组难记和弦：`Ctrl+T` 思考、`Ctrl+O` 工具视口、`Alt+E` 块展开、`Alt+Shift+E` / `Ctrl+Alt+Shift+E` 最近活动段、`Alt+I` / `Alt+U` / `Alt+O` / `Alt+Shift+U` 详情与侧栏。屏上回显又把这些键名写进正文（`Thinking  (Ctrl+T)`、`ctrl+o to expand`、`Alt+Up to edit all queued messages`），用户既要记住键，又要读回显。

鼠标当前只能点 transcript 里的折叠块与待办欄翼头。所有 overlay 列表（树 / resume / models / themes / MCP / capabilities / import / ask）纯键盘；详情与侧栏只能关、不能开。

目标：**鼠标加滚轮覆盖全部常用操作，键盘退为安全网与高级加速器**。

## What Changes

### 1. 键位收缩（action 保留，默认绑定清空）

- `app.thinking.toggle`、`app.tools.expand`、`app.tools.blocks`、`app.activity.expandNearest`、`app.activity.collapseNearest`、`app.session.popup`、`app.session.sidebar`、`app.session.detailsCycle`、`app.session.sidebarWidth` 九条 action 的默认键 全部清空。
- action id 与分发代码保留：用户可在 `~/.xylitol/keybindings.json` 自行重绑，未绑定时不响应任何键。
- 保留不改：`Esc` 中断、`Ctrl+C` 清空/退出、`Enter` 提交、`Alt+Enter` follow-up、`Alt+Up` 取回队列、`Ctrl+G` 外编、`Ctrl+V` 图片粘贴、树与 resume 的过滤/标注/排序键（它们已在槽内，且鼠标入口随本 change 补齐后保留为加速器）。
- 空默认绑定已在 `KeybindingsManager` 受支持（`tui.select.pageUp` 现为空），无需引擎新能力。

### 2. 新增可复用基础（前人栽树）

- 引擎新增 `ActionRow`：一行 chip 的渲染 + 每段列区间 + 激活态与禁用态；不得持有业务语义。对齐 `Tabs::title_col_ranges` 的既有形态。
- 应用端新增 dock 动作命中表：每次渲染登记 `DockActionHit { row, col_start, col_end, action }`，点击在 `UiRoot::handle_pointer_input` 先于待办欄命中分派。以后新增任何固定区按钮 只需「渲染 chip + 推一条 hit + 加一个 `DockAction` 变体」。
- 队列条的 `QUEUE_STRIP_HIT_ID` 命中区不再依赖 transcript 命中管线（它在 dock 内，屏幕行被 `transcript_rows` 上限挡掉）；改为同一条 dock 动作命中。

### 3. 屏上动作

- footer 右端新增视图 chip 组：`详情`、`侧栏`；侧栏开启时 `侧栏` chip 反色；侧栏宽度仍点分隔条循环。窄档按 `侧栏` chip 降级为详情弹层（与既有降级同口径）。
- 存在折叠覆盖时 footer 右端额外出现 `重置折叠` chip，等价旧 `Alt+E` 的「翻转默认态 + 清空 per-id 覆盖」。
- 块头回显去键名：`Thinking (Ctrl+T)` → 只画折叠三角与标签；`ctrl+o to expand/fold` → 中性 `click to expand/fold` 语义文案；`Alt+E to expand` → 同前；活动段头不再拼 `(Alt+Shift+E)`。
- 「最近活动段」不再有鼠标缺口：屏幕上看到的段就是「最近」段，点它即 toggle；`expandNearest` / `collapseNearest` 改为无默认绑定的高级 action。

### 4. overlay 槽鼠标化

- 树 / resume / models / themes / MCP / capabilities / import / ask 支持指针：单击行 = 选中；**再点同一已选中行 = 执行该行主动作**（不用双击计时）。
- 每个槽头部右侧画 `关闭` chip；import 确认画 `是` / `否` chip；ask 画 `提交` / `跳过` chip。
- capabilities 行点击 = 直接 toggle（开关可逆，一步到位）。

### 5. 待办欄改版

- 默认只展开 doing 列；pending / completed 默认只留翼头计数行，点翼头展开。
- 高度上限与 `↑ N more` 边框、正文可划选复制、三角列不可选区等既有条款不变。
- 不改：待办条目不可由用户点改状态（仍只读）。

## Capabilities

- 新增 `app-tui-mouse-actions`：鼠标动作覆盖与键位收缩的产品合约。
- 修改 `app-tui-fixed-zone`（footer 动作 chip、待办欄默认折叠、详情/侧栏切换入口）。
- 修改 `app-tui-input`（键位目录与安全网划分）。
- 修改 `app-tui-transcript`（块头回显、活动段提示）。
- 修改 `agent-todo`（待办欄缺省展开面）。
- 包实例：新增 `package-tui-action-row`；修改 `package-tui-pointer`（dock 绝对行命中）。

## Impact

- `crates/xylitol-tui`：新增 `components/action_row.rs` 与导出；`pointer.rs` 增 dock 命中空间。
- `crates/xylitol-app-tui`：`keybindings.rs`（9 条默认键置空）、`layout/root/{render,slot_input,mod}.rs`（footer chip 与 dock 命中）、`layout/slots/*`、`session_resume/panel.rs`、`widgets/scrollback/*`（回显）、`widgets/todo_bar.rs`（缺省折叠）。
- 测试：应用端 harness/tests 里按旧和弦驱动的用例改为按 action 直接调用或鼠标事件；新增 dock chip 与槽行点击用例；BDD bindings 同步。
- 不碰线协议、Host 侧与 Print 端。

## Further Notes

- 设计决策与落点：[design.md](./design.md)。
- 鼠标悬停高亮本轮不扩到 dock（Moved 在 dock 内不到达应用层）；chip 靠括号形态与激活反色表达可点。
