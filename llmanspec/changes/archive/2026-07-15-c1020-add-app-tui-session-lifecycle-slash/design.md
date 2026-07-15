# Design — c1020-add-app-tui-session-lifecycle-slash

## pi 行为调研

### `/new` → `/session-new`

- `handleClearCommand` → `runtimeHost.newSession()`：新建空会话文件并替换当前会话；成功提示 “New session started”。
- 无参；无确认对话框（extension 可 cancel）。

**xylitol**：Driver `new_session` = `store.create(uuid)` + `set_session`；UI 清空 transcript（空 entries rebuild）+ 系统行。旧名 `/new` 无效。

### `/clone` → `/session-clone`

- `handleCloneCommand`：取 leaf；无 leaf → “Nothing to clone yet”；有则 `runtimeHost.fork(leafId, { position: "at" })` 后切到新会话。
- **刻意不同于** pi `/fork`（user 选择器）与 xylitol `/session-fork`（user→Before / 非 user→At）。

**xylitol 钉死**：clone **恒** `ForkPosition::At` + switch；复用 `fork_session`；UI 可复用 fork 后 rebuild，文案用 “Cloned…”。记入 `PI_DELTAS` **A07**。

### `/name` → `/session-name`

- 无参：有名则显示，否则 `Usage: /name <name>`。
- 有参：`appendSessionInfo`；sanitize = 换行→空格 + trim；若规范化后与输入不同则 warning。

**xylitol**：Driver `get_session_name` / `set_session_name`；底层已有 `SessionManager::get_session_name` / `append_session_info`（可经 store 默认实现或 Driver 拼 `SessionInfo` entry）。旧名 `/name` 无效。

## Seam

| 方法 | 说明 |
|---|---|
| `Driver::new_session` | 新建空会话并设为当前；返回 id |
| `Driver::get_session_name` | 当前会话 display name |
| `Driver::set_session_name` | sanitize 后写入；返回最终名 |
| `fork_session(..., At)` | clone 复用 |

均 **Driver-only**（非 `protocol::Command`），对齐 `list_sessions` / `load_debug_scene`。Remote 可 stub Err。

## 非目标

- REST `/sessions` 新建对称；列表内 rename；改 A02 fork 规则。
