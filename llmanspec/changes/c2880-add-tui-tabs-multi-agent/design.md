# Design — 包层 Tabs

## 范围

本票只加 `packages/xylitol-tui` 的 `Tabs`。产品 host、`src/app/tui`、Down 重绑、子 agent runtime 都不改。

## 决策

| id | 内容 |
|---|---|
| D1 | 只做刀 A。刀 B（产品 Down → picker / 子面）park。 |
| D2 | 组件名 `Tabs`。状态是 `titles: Vec<String>` + `selected: usize`。不持 session id。 |
| D3 | page 可选：`pages: Vec<Box<dyn Component>>`。`pages.get(selected)` 有值才往下拼它的 `render`。未选中的 page 不调用 `render`，也不出现在输出里。 |
| D4 | 实现 `Component`。需要收键时实现 `Focusable`。host / `TUI` 决定谁有焦点。 |
| D5 | 键走 `KeybindingsManager`，仿 `SelectList` 的 `with_keybindings`。新 id：`tui.tabs.prev` 默认 `left`，`tui.tabs.next` 默认 `right`。循环。 |
| D6 | `1`–`9`（无修饰 `Char`）选 1-based 下标。`n > titles.len()` 忽略。不进 keybindings 表。 |
| D7 | 鼠标：`handle_input` 忽略。`title_col_ranges()` 只暴露几何。不登记 `HitRegion`。 |
| D8 | 标题条一行。分隔符一个空格。总宽超过 `width` 时按下面算法截断，保证当前 title 可见。 |
| D9 | `Vec<String>` ANSI。主题用闭包。不用 `StyledLine`，不依赖 ratatui `widgets::Tabs`。 |
| D10 | `Container` 仍只垂直排。`Tabs` 自己叠「条 + 当前 page」。 |
| D11 | 选中变化可回调 `on_change: Option<Box<dyn FnMut(usize)>>`。crate / BDD 用它断言，不是产品事件。 |

## 公开形状

```text
Tabs::new(titles: Vec<String>)
  .with_pages(pages)
  .with_theme(TabsTheme)
  .with_on_change(f)
  .set_selected(i) / selected() / set_titles(...)
  .title_col_ranges() -> Vec<(usize, usize)>   // [start, end) 列，相对条行
```

`set_titles` 之后若 `selected >= titles.len()`，夹到 `len.saturating_sub(1)`（空表则 0）。

空 `titles`：`render` 返回空 `Vec`，`title_col_ranges` 空。

## 布局

1. 量每个 title 的显示列（按现有 `visible_width` / 截断工具，与 `SelectList` 同一套）。
2. 条 = `title0 + " " + title1 + ...`，总目标宽 = `width`。
3. 若合计（含分隔空格）≤ `width`：按自然宽排，多余列留在右侧空白。
4. 若超过：每个 title 至少留 1 列；先给 `selected` 尽量完整宽，剩余均分给其他 title（不够则再截 selected）。分隔空格优先保留；宽 < 2N-1 时丢掉部分分隔，仍保证 selected 至少 1 列。
5. 每个 title 的 `[start, end)` 写入 `title_col_ranges`。分隔空格不计入任一区间。
6. 下一行起拼 `pages[selected].render(width)`，不再套边框。

## 输入

`handle_input` 只处理 `InputEvent::Key`：

- `tui.tabs.prev` → `selected = (selected + n - 1) % n`
- `tui.tabs.next` → `selected = (selected + 1) % n`
- `Char('1'..='9')` 且无 ctrl/alt/shift → 若 `i <= n` 则 `selected = i-1`
- 其余：`InputResult::NotHandled`（含鼠标、`tui.select.*`）

`n == 0` 时所有切页键 `NotHandled`。

切页成功：`invalidate`，调 `on_change`，`InputResult::Handled`。

## 主题

`TabsTheme` 最少两个闭包：

- `title_style(selected: bool) -> Option<Style>`
- `divider_style() -> Option<Style>`（可选；默认无色空格）

不在包里写死产品色。

## 键位与产品冲突

`tui.select.pageUp/pageDown` 默认未绑 left/right（c1470，`/model` 切档）。`Tabs` 默认占用 left/right **仅当自己有焦点**。本票不改 editor、不改 `/model`。产品以后把 `Tabs` 嵌进 host 时，必须自己决定焦点，不能假定全局 left/right 属于 tab。

## 测试缝

复用包 `Component::handle_input` / `render`，以及仓库 `tests/bdd` 的 rstest-bdd 步骤（`bindings_package_tui_*` 模式）。不新开 harness。

| 缝 | 断言 |
|---|---|
| crate `tabs` 单测 | left/right 循环；`1`–`9`；越界 digit 忽略；未选 page 不出现；`title_col_ranges` 落在 `width` 内且不重叠 |
| BDD `package-tui-tabs` | 同上行为，经 `tests/bdd/bindings_package_tui_tabs.rs` |

`agent_demo` 不作为本票完成条件。

## 规格映射

| req | 行为 |
|---|---|
| r1938 | 键切 `selected`（左右循环、digit） |
| r1939 | 只渲染当前 page |
| r1940 | `render` 后 `title_col_ranges` 可用 |

## 明确不做

- `HitRegion` / `dispatch_event(Mouse)` / hover
- 多行标题条、垂直 tab
- 产品 host 接线、跨端同源新词
- 子 agent 编排
