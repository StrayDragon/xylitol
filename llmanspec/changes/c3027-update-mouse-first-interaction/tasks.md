# Tasks

## 包：dock 命中与动作 chip 基础

- [ ] 1.1 `HitSpace::Dock` + `Hit::at_screen`：dock 绝对行命中不受 `transcript_rows` 与视口滚动影响；`Content` / `Viewport` 行门不变；`apply_hover_tint` 仍只处理 transcript 行。单测：登记 dock 命中后按超出 transcript 的绝对行查询可命中，滚动不改结果。
- [ ] 1.2 `ActionRow` 组件：`Vec<ActionChip { label, active, disabled }>` → 单行 + `chip_col_ranges()`；宽度不足从尾部整 chip 丢弃；导出到 `lib.rs`。单测：区间落界内且互不重叠；active 反色；disabled 不进区间表。
- [ ] 1.3 列表行映射：`SelectList` / `TreeSelector` / `ChoicePrompt` 暴露「屏行 → 项下标」查询（含 `set_screen_origin` 偏移与滚动起点），不改变既有键盘行为。单测：设 origin 后按屏行取到正确下标，越界返回 None。
- [ ] 1.4 BDD 包场景：`package-tui-pointer` 增 `dock-hit-absolute-row` 规则与场景；新建 `package-tui-action-row` capability（规则 + 场景）。`llman-sdd validate` 在包根跑一次（`--check`，不用 `--strict`）。

## 应用：屏上动作与键位收缩

[blocked-by: 1.1, 1.2]

- [ ] 2.1 footer 右端视图 chip：footer 改左正文 + 右 `[详情]` `[侧栏]`（标准档两者，窄档只 `[详情]`），侧栏开启 / 弹层开启时对应 chip 反色；窄档点 `[侧栏]` 走既有降级开弹层。单测：渲染含 chip；窄档只剩 `[详情]`。
- [ ] 2.2 dock 动作表与派发：每次渲染重建 `dock_hits`；`handle_pointer_input` 先查 dock 动作再查待办栏；队列条改登成 `DequeueQueue` 并删掉不可达的 `QUEUE_STRIP_HIT_ID` chrome 登记。单测：注入 chip 位置 Down/Up 后状态变化；队列条点击仍能 dequeue。
- [ ] 2.3 `[重置折叠]` chip：存在 per-id 覆盖或非默认默认态时出现（标准档及以上）；点击等价旧 `Alt+E`（翻默认态 + 清覆盖）。单测：无覆盖时不渲染；有覆盖时点击后覆盖表空。
- [ ] 2.4 键位收缩：`app.thinking.toggle`、`app.tools.expand`、`app.tools.blocks`、`app.activity.expandNearest`、`app.activity.collapseNearest`、`app.session.popup`、`app.session.sidebar`、`app.session.detailsCycle`、`app.session.sidebarWidth` 九条默认键置空，action 与分发保留：单测断言空默认键不匹配任何键（含 `~/.xylitol/keybindings.json` 重绑后仍生效）。
- [ ] 2.5 旧和弦测试改写：`tests.rs` 中 `harness_app_thinking_toggle_via_binding_id`、`scrollback_thinking_fold_shows_ctrl_t_hint`、`tools_per_block_override_and_alt_e_clears`、`thinking_per_id_override_and_ctrl_t_clears`、`activity_fold_att27/att28`、`scrollback_bash_ctrl_o_viewport_keeps_rail` 等改为直接调 `toggle_fold_target` / `expand_nearest_activity` 或重绑后再按键；断言旧和弦不再触发。`harness.rs` 中树过滤与 resume 键不动。
- [ ] 2.6 回显去键名：`(Ctrl+T)`、`(Alt+E)`、`(Alt+E to expand)`、活动段 `(Alt+Shift+E)` 删除；`ctrl+o to expand/fold` → `click to expand/fold`；队列条提示 → `↳ click to edit all queued messages`；`Running… (Esc)` 保留。同步 `scrollback/tests.rs` 与 `tests.rs` 中的回显断言。
- [ ] 2.7 待办栏缺省面：`todo_past_open` / `todo_pending_open` 缺省 `false`（doing 仍 `true`）；翼头行仍全画。单测：默认渲染只展开 doing 正文。

## 應用：槽鼠标化

[blocked-by: 1.3]

- [ ] 3.1 槽头 `[关闭]` chip 统一：树 / resume / models / themes / MCP / capabilities 头行右侧各画一个；点击关槽。单测：每槽点击后回到 `EditorSlot::Editor`。
- [ ] 3.2 行点击选中 + 二次点击执行：树（travel）、resume（切会话）、models（选模型）、themes（选主题）、MCP（关）、scenes（选场景）；不做双击计时。单测：首次点只改选中，再次点触发动作。
- [ ] 3.3 特殊面：import 画 `[是]` `[否]`；ask 画 `[提交]` `[跳过]` 并在选项行支持点选；capabilities 行点击直接 toggle。单测：各自状态变更。
- [ ] 3.4 BDD 应用场景：`app-tui-fixed-zone` 增 footer chip 与待办栏缺省面场景；`app-tui-input` 增键位目录收缩场景；`app-tui-transcript` 增回显无键名场景。步骤实现复用途既有 `TranscriptBdd` / host 泵，不另造副作用泵。

## 规格与门禁

- [ ] 4.1 新增 `app-tui-mouse-actions` capability：鼠标动作覆盖、键位收缩与安全网划分、槽行点击语义、回显无键名。
- [ ] 4.2 改 `app-tui-fixed-zone` r2903 / r2904（详情与侧栏的屏上出入口）与 r21（待办栏缺省面）。
- [ ] 4.3 改 `app-tui-input` r1286（队列条提示）与键位目录相关规则（新增「默认绑定收缩」条款）。
- [ ] 4.4 改 `agent-todo` r1129（缺省展开面）。
- [ ] 4.5 改 `crates/xylitol-tui/llmanspec/specs/package-tui-pointer`（dock 命中）并新建 `package-tui-action-row`。
- [ ] 4.6 全量验证：`cargo nextest run -p xylitol-tui -p xylitol-app-tui`、`just test-tui`、`just fmt` + `cargo clippy -p xylitol-tui -p xylitol-app-tui --all-targets`、`llman-sdd validate c3027-update-mouse-first-interaction`。
- [ ] 4.7 真终端抽查：`just test-tui-e2e-pty`，并手测一次鼠标点 footer chip / 槽行 / 待办栏翼头。

## 文档

- [ ] 5.1 更新 `crates/xylitol-app-tui/src/DESIGN.md`：硬规则 2 例外改为「经 footer chip 或侧栏分隔条进出」；布局段 footer 画法改为左右两段；键位措辞去除已删和弦。
- [ ] 5.2 更新 `crates/xylitol-app-tui/src/AGENTS.md` 与根 `AGENTS.md` 中提及已删和弦的句子（如有）；`_HANDOFF/tui-interaction-inventory.md` 标注已被本 change 取代。
