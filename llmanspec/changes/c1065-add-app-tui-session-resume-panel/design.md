# Design — c1065-add-app-tui-session-resume-panel

## pi 对照（源）

`../pi/packages/coding-agent/src/modes/interactive/components/session-selector.ts`
+ `session-selector-search.ts`（`parseSearchQuery` / SortMode / NameFilter）。

面板在 **coding-agent**（产品），不在 pi-tui 包 → xylitol 亦落在 `src/app/tui/`，包层只复用 Input/SelectList/键位原语。

## 布局

```
┌ Resume Session (Current Folder)     ◉ Current | ○ All   Name: All   Sort: Threaded ┐
│ tab scope · re:<pattern> · "phrase" exact                                           │
│ ctrl+s sort · ctrl+n named · ctrl+d delete · ctrl+p path · ctrl+r rename            │
│ > filter…                                                                           │
│ › first user message or name                                          12  6m        │
│   └─ child preview                                                     3  1h        │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

仍为 **EditorSlot::SessionResume**（替换贴底 editor），非居中 overlay。

## 钉死决议

| 主题 | 决议 |
|---|---|
| Scope Current | `SessionHeader.cwd` 与当前进程 cwd（canonicalize）匹配的会话 |
| Scope All | Driver 能列举的全部可 resume 会话（今日单 `sessions_dir` 即该目录全部；若未来多 project 子目录，seam 扩展，TUI 不直读） |
| Sort Threaded | parent/child forest + 前缀；同级 mtime 降序 |
| Sort Recent | 扁平 mtime 降序（无前缀） |
| Sort Fuzzy/relevance | 有搜索 query 时按匹配分；空 query 时等同 Recent |
| Rename | 面板内短 Input；确认后 `Driver::set_session_name`；列表就地刷新 name |
| Delete | 确认态（Enter 确认 / Esc 取消）；`Driver::delete_session`；当前 session MUST 拒绝并提示 |
| Fold（P2） | 仅 Threaded；折叠状态为面板本地 UI 态（不写盘）；键：`tui.tree.foldOrUp` / `tui.tree.unfoldOrDown`（与会话树一致） |
| Live N/M | **本 change 不做**；保留 Loading 占位 + OSC；见 future |

## Seam 增量

| API | 用途 |
|---|---|
| `list_sessions` 扩展字段 | `cwd` / `path`（展示与 scope） |
| `delete_session(id)` | P1 删除 |
| （已有）`set_session_name` / `switch_session` | rename / Enter |

Remote：未实现可 Err，产品 InProcess 必绿。

## 模块落点

- `src/app/tui/layout/session_resume.rs`（或 `widgets/session_resume.rs`）：面板状态机
- `commands` / `effects`：仍只负责打开/关闭与 Driver 调用
- 搜索解析可本地移植 pi `parseSearchQuery`（Rust），单测覆盖

## 刻意差异（若保留）

相对 pi 多 project 全局 `listAll`：若 xylitol 仅单 `sessions_dir`，All=该目录全量 —— 记 `PI_DELTAS` 一行，不得静默假装多根。
