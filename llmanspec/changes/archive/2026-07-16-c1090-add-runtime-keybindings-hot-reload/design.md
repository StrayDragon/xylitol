# Design — c1090-add-runtime-keybindings-hot-reload

## Decision

### 1. 包 API

- `KeybindingsConfig`：`HashMap<String, Vec<String>>`（JSON 友好）
- `keys_by_id` 解析结果存 `Vec<String>`；`matches_event` 仍调 `matches_key_event(&str)`
- definitions 仍以 `&'static str` id 为权威目录；未知 JSON 键忽略（可诊断 warning）
- 保留 `set_user_bindings` / 全局 `set_keybindings` / `with_keybindings`

### 2. 产品目录（本 change 必注册）

对齐 pi 命名，默认键与当前硬编码一致：

| id | default | 用途 |
|---|---|---|
| `app.interrupt` | escape | abort / Esc 策略 |
| `app.clear` | ctrl+c | 清编辑器 |
| `app.message.followUp` | alt+enter | follow-up |
| `app.message.dequeue` | alt+up | 队列还原 |
| `app.editor.external` | ctrl+g | 外部编辑器 |
| `app.thinking.toggle` | ctrl+t | thinking 折叠（树关） |
| `app.tools.expand` | ctrl+o | 工具视口（树关） |
| `app.tools.blocks` | alt+e | tool/diff 块（产品 Alt+E） |
| `app.tree.filter.default` | ctrl+d | |
| `app.tree.filter.noTools` | ctrl+t | |
| `app.tree.filter.userOnly` | ctrl+u | |
| `app.tree.filter.labeledOnly` | ctrl+l | |
| `app.tree.filter.all` | ctrl+a | |
| `app.tree.filter.cycleForward` | ctrl+o | |
| `app.tree.filter.cycleBackward` | ctrl+shift+o | |
| `app.session.fork` | shift+f | 树内 fork |
| `app.tree.editLabel` | shift+l | （可与包 `tui.tree.editLabel` 二选一：产品统一用 app.*） |
| `app.tree.toggleLabelTimestamp` | shift+t | |
| `app.session.toggleSort` | ctrl+s | Resume |
| `app.session.toggleNamedFilter` | ctrl+n | Resume |
| `app.session.togglePath` | ctrl+p | Resume |
| `app.session.rename` | ctrl+r | Resume |
| `app.session.delete` | ctrl+d | Resume |

树 fold 继续用已有 `tui.tree.foldOrUp` / `unfoldOrDown`。Enter/Esc 在选择器场景继续 `tui.select.*` / `tui.input.submit`。

### 3. 装配与重载

- 启动：`create_default_definitions()` ∪ 产品 `APP_KEYBINDINGS` → `KeybindingsManager::new` → `set_keybindings`；再读 `agent_dir/keybindings.json` 覆盖
- `reload_keybindings()`：读盘 → 解析 → 成功则 `set_user_bindings`；失败保留旧 `keys_by_id` 并返回诊断
- 通知：同步回调 / EventBus `keybindings:changed`（供 TreeHelp 失效）；TUI host MUST NOT 因重载清 session

### 4. 匹配迁移

产品路径禁止新增字面和弦匹配（测试里构造 KeyEvent 除外）。`slot_input` / `input_policy` / `session_resume` / host Ctrl+C·Esc 改 `with_keybindings(|kb| kb.matches_event(...))`。

## Trade-offs

- 同 change 含重构 + 热重载：PR 偏大，但避免「只能重载 tui.*」的半成品。
- `app.tools.blocks` 非 pi 名（pi 无 Alt+E 对等）：刻意产品 id，记 PI_DELTAS。
- agent_demo 本 change 不强制全改（MAY）；产品 harness 覆盖回归。
