# Tab / 多 agent 对照

## xylitol

- 无 `Tab` 组件。`EditorSlot`：Editor / Tree / SessionResume / Models / …
- Down：editor 光标或 prompt-history；槽内 `tui.select.down`。树展开是 Ctrl/Alt+Right，不是 Down。
- 子 agent：roadmap only。`src/AGENTS.md`：未来子 agent = 另建隔离 runtime。

## pi

- 无 in-app tab。SessionSelector / TreeSelector 与 xylitol 同族。无 Down-to-child。

## opencode

- `session-tabs.tsx` + `SessionTabsProvider`；keymap `session.tab.next` = Ctrl+Tab / Alt+Down。
- **Down** = `session.child.first`「Toggle subagent picker」。Up = parent；Left/Right = 兄弟 child。
- 子 agent = 带 `parentID` 的 child session，不是泛型 Pager。

## crush

- 无 tab 条；会话是 dialog/list。

## ratatui

- `ratatui-widgets` `Tabs`：纯 paint（`select` + divider + padding），不处理键/鼠标。Demo 用 Left/Right。
- 点击 = app 按 title 宽度切 `mouse.column`（widget 不返回每 tab Rect）。
- ratatui-markdown hybrid scroll 是键盘 engaged 焦点，不是 Down-enter 子 agent。
