---
version: "alpha"
name: "keybindings"
description: "Resolved product key chords — parenthesized hints in UI."
tokens_from: "../DESIGN.md"
components:
  key-hint:
    textColor: "{colors.muted}"
---

# Keybindings

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 已接线：c480 / c615 / c630 `/model` / **c1115 `/theme`**。下一波：c635–c645 树 power · c650 真 `$EDITOR`。
> 活实验场：`just demo-tui`。静图：[`playground/`](./playground/)。

## MUST — 全局 / 输入（已落地）

| 键 | 行为 |
|---|---|
| Esc | 流中：**abort**（清 steer，**留** follow_up 供 restore） |
| Ctrl+C | 编辑器非空：**清空**；已空：**退出** TUI |
| Enter（idle） | 提交用户消息 |
| Enter（流中） | **steer** |
| Alt+Enter | **follow-up**（排队到本轮结束后） |
| Alt+Up | 将已排队 steer/follow-up **还原进 editor** 并清空两侧队列 |
| ↑ / ↓（editor） | 在首/末可视行且（空草稿或已在浏览）时：**发送历史**召回（c481；包 ed05） |
| 双 Esc | 打开 **MessageHistory 活树**（`session_tree`；Esc 关；Enter：`travel_session_tree`，user→`editor_text` 预填） |
| `/exit` | 退出并 restore（与垂直切片一致） |
| `/model` | 打开 **fuzzy 模型列表**（替换 editor 槽；见 [`models-picker.md`](./models-picker.md)；**c630**；对齐 pi） |
| `/model <id>` | 直选模型（不经列表） |
| `/theme` | 打开 **Themes 槽** SelectList（`dark` / `light`；见 [`theme-tokens.md`](./theme-tokens.md)；**c1115**） |
| `/theme dark` \| `light` | 经 `HostSession::reload_themes` 直切色板 |
| `/theme toggle` \| `cycle` | dark↔light 翻转（相对当前 `theme_preference`，无则视为 dark） |

**移除**：无参 `/model` 静默 **cycle** — 改为打开列表；**不**引入 `/models`。

**产品不抄**：demo Ctrl+P `theme-toggle`（主题只走 `/theme` slash）。
## MUST — 树关时（有内容时）

| 键 | 行为 |
|---|---|
| Ctrl+O | 工具详情视口折叠/全文 |
| Ctrl+T | thinking 展开/折叠 |
| Alt+E | tool/diff **块**展开/折叠 |
| Shift+Tab | **仅 `/model` picker 开且焦点模型可调思考时**：cycle xylitol 等级（与 ←→ 同槽，见 [`models-picker.md`](./models-picker.md)）。**MUST NOT** 全局 cycle thinking |
| ←→ | **仅 `/model` picker 开且焦点可调时**：在支持集上移动暂定等级（循环） |

## MUST — 树开（产品接线 · c635→c645；demo 已有）

| 键 | 行为 | Change |
|---|---|---|
| Ctrl+D | filter → default | **c635** |
| Ctrl+T/U/L/A | filter **toggle** ↔ default（no-tools / user / labeled / all） | **c635** |
| Ctrl+O | filter cycle forward（树开优先；关树仍为工具视口） | **c635** |
| Ctrl+Shift+O | filter cycle **backward** | **c685** |
| Ctrl/Alt+←→ | fold / 分支跳转（转发包）。裸 ←→ **不**翻页（`tui.select.pageUp|pageDown` 默认无键，留给 `/model`） | **c640** / **c1470** |
| Shift+F | **fork** 新 session（user→Before / 非 user→At；对齐 pi） | **c645** |
| Shift+L / T | annotation 编辑 / 时间戳显隐 | **c690** |
| Enter | `travel_session_tree`（已 **c615**） | — |

树槽头行：**Help**（⊞⊟ fold 等用途）+ **Search**（键位经 KeybindingsManager / 产品 filter 和弦；**c685**）。口语勿称「chrome」（易与浏览器混淆；layout 壳合约 id 仍为 `app-tui-chrome`）。

## MUST（编辑器槽）

| 键 | 行为 |
|---|---|
| Esc（选择器打开时） | 关闭选择器，还原 editor |
| `/` 补全 | CompletionSource；产品命令含 `/exit` `/model` `/theme` `/session` `/session-resume` `/session-tree` `/session-fork` `/session-compact` `/session-export` `/session-import` `/reload` `/trust` `/history-copy-last`（**c1005–c1115**）；debug 构建另有 `/debug`；`/theme ` 参数补全至少 `dark`/`light`（可含 `toggle`） |
| `/session-tree` | 打开会话树（同双 Esc；**c700/c1005**；旧名 `/tree` 无效） |
| `/session-fork` | 在当前 leaf fork（同 Shift+F 语义；选节点仍用树；**c700/c1005**；旧名 `/fork` 无效） |
| `/session` | 转储会话 info/stats（**c1015**；非操作菜单） |
| `/session-resume` | 会话 Resume 面板（scope/sort/搜索/rename/delete；**c1065**；旧名 `/resume` 无效） |
| `/session-compact` | 手动 Compact（**c1010**；仅无参） |
| `/session-export` [path] | 默认 HTML；`.jsonl` → JSONL（**c1010**） |
| `/session-import` \<path\> | Yes/No 确认后 ImportJsonl（**c1010**） |
| `!` / `!!` 前缀 | bash 边框 + idle Enter → `execute_bash`（**c492**） |
| Ctrl+G | 外部编辑器（**c650**）：TTY + 已配置 `$VISUAL`/`$EDITOR` → 真编辑器；harness / 非 TTY → stub；未配置/失败 → `UiEntry::Error`（无静默默认编辑器） |

## MUST — Resume 面板（`EditorSlot::SessionResume` · c1065）

| 键 | 行为 |
|---|---|
| Tab | scope **Current** ↔ **All** |
| Ctrl+S | Sort 循环：Threaded → Recent → Fuzzy |
| Ctrl+N | Name filter：**All** ↔ **Named** |
| Ctrl+P | 切换行内 path/cwd 显示 |
| Ctrl+R | 重命名选中项（Enter 确认；Esc 取消） |
| Ctrl+D | 删除确认（Enter 确认；Esc 取消；**禁止**删当前活跃 session） |
| Ctrl/Alt+←→ | Threaded 下折叠/展开父节点的子会话 |
| Enter | switch 选中会话 |
| Esc | 取消 rename/delete 子态，或关闭面板 |
| 可打印键 | 搜索 filter（`re:` / `"phrase"` / fuzzy token） |

## 明确不做（键位）

- Settings / Plate 槽的运行时配置编辑（配置走 YAML+JSON Schema）；**产品 MUST NOT** 绑定 Ctrl+P 打开 Command Plate stub（demo 可有）。
- computer-use 专用键位（本波延后）。
- 产品 `/thinking-level` slash；**全局** Shift+Tab / `app.thinking.cycle`（thinking 只在 `/model` picker 内配置，见 [`models-picker.md`](./models-picker.md)）。

## 规则

1. 全局键经 `InputListener` **先于** Editor 焦点消费（c455）。
2. **MUST NOT** 让 Ctrl+C 泄漏进 Editor 变成字面 `c`。
3. footer **默认不**罗列完整快捷键墙；细节 `/help`（若有）。
4. UI 旁注快捷键 MUST 用括号包裹完整和弦（如 `(Ctrl+T)`、`(Alt+E)`），**MUST NOT** 使用 `^T` 缩写；提示色 `{colors.muted}`。
