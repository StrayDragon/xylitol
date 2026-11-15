# Design — 空输入提示行与快捷键面板

## Goals

- 空输入可发现 `?` 入口；按 `?` 打开快捷键面板。
- 不改 `Editor`，不加 chip 命中；面板内容随 keybindings 自定义。
- footer 第一行不变；第二行 hint 仅在需要时出现。

## Non-goals

- 点击 hint 或面板行触发动作。
- `/help` slash 命令。
- 面板写入会话历史或持久化。

## Layout and footprint

- dock 顺序：queue、todo、toast、status、tray、editor、footer、hint。
- hint 行 0/1：
  - editor 空且面板关闭：`? shortcuts`
  - 面板打开：`? / Esc close`
  - editor 非空：0 行
- hint 行 dim；`?` 用 accent，其余 muted。
- `last_dock_rows` 与 `editor_top_row` 必须计入 hint 行。
- `Fixed-Zone Footprint` 的预留行必须加入 hint 0/1 行。
- 面板打开时 upper area 渲染面板；loaded resources 与 scrollback 隐藏。
- 面板高度按可用 transcript 行计算；内容过高时滚动。

## State

- `help_open: bool`（默认 false）
- `help_scroll: usize`（默认 0）
- 切换时 invalidate upper cache；不写入 UiModel。

## Key handling

- `handle_editor_keys` 在 `editor.handle_input` 之前拦截：
  - 面板打开：Esc 或 `?` 关闭；Up/Down/PageUp/PageDown 滚动；其它键忽略。
  - 面板关闭且 editor 空：`?` 打开面板，consume。
  - 面板关闭且 editor 非空：`?` 交给 Editor，输入字面 `?`。
- `on_escape`：面板打开时先关面板并重置双 Esc 窗口；不得 abort。
- `on_ctrl_c`：面板打开时先关面板；不得清空输入或退出。
- busy 时 `?` 仍可打开；Esc 先关面板，第二次 Esc 才 abort。
- `?` 判定用 `KeyCode::Char('?')`，不依赖修饰键。

## Panel content

- 静态分组 + 动作 id；键位从 `with_keybindings(|kb| kb.get_keys(id))` 取当前值。
- 分组：
  - 输入：`app.interrupt`、`app.clear`、`app.editor.external`、`app.paste.image`、`app.message.followUp`、`app.message.dequeue`、`tui.input.newLine`、`tui.input.submit`。
  - 会话：`app.session.fork`、`app.tree.editLabel`、`app.tree.toggleLabelTimestamp`、`app.session.rename`、`app.session.delete`、`app.session.toggleSort`、`app.session.toggleNamedFilter`、`app.session.togglePath`、`app.session.toggleId`。
  - 显示与槽：`app.slot.pageUp`、`app.slot.pageDown`、`tui.tree.foldOrUp`、`tui.tree.unfoldOrDown`、`app.tree.filter.*`。
- 非 keybinding 的静态入口：`/` commands、`@` files、`$` skills、`!` shell、`Esc Esc` session tree、`?` shortcuts。
- 键位格式化复用 `layout/session_tree.rs` 的 `pretty_key_id` / `compact_raw_keys`（必要时提升为共享 helper）。
- 标签用中文映射；未知 id 回退到 `KeybindingDefinition.description`。
- 列数：width ≥ 100 三列；≥ 70 两列；否则单列。
- 滚动：Up/Down 一行；PageUp/PageDown 一屏；clamp 到内容行数。
- 面板边框与标题：`╭─ 快捷键 ─╮`；不写 `[关闭]` chip，关闭提示走 hint 行。

## Rendering

- `UiRoot::render` 在 `help_open` 时跳过 loaded resources 与 scrollback，渲染面板。
- 面板行数不超过可用 transcript 行；不足时补空行或留白。
- 切换 `help_open` 时 `bump_upper_gen()`，避免旧 upper cache。
- 面板不进入 `UiEntry` / scrollback / 会话历史。

## Specs impact

- `app-tui-fixed-zone`：新增 `footer-hint-line`（r3356）；更新 r1228 footprint 加入 hint 0/1 行。
- `app-tui-input`：新增 `help-panel-shortcut`（r3357），覆盖 `?` 优先级、Esc/Ctrl+C 优先级、busy 优先级。
- `app-tui-commands`：不改；`/help` 不在本期。
- `app-tui-mouse-actions`：不改；hint 不要求指针入口。

## Risks

- 输入首个字符时 hint 行消失，编辑区上移 1 行；Codex 同形，接受。
- 空输入不能以 `?` 开头；必须先输入空格或其它字符。
- 面板内容过多时依赖滚动；窄终端列数降级。
- 动态键位冲突已由 reload 诊断覆盖；面板只显示当前生效键位。
