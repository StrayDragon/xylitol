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
> 已接线：c480 / c615。下一波：c630 `/model` 列表 · c635–c645 树 power · c650 真 `$EDITOR`。
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

**移除**：无参 `/model` 静默 **cycle** — 改为打开列表；**不**引入 `/models`。

## MUST — 树关时（有内容时）

| 键 | 行为 |
|---|---|
| Ctrl+O | 工具详情视口折叠/全文 |
| Ctrl+T | thinking 展开/折叠 |
| Alt+E | tool/diff **块**展开/折叠 |

## MUST — 树开（产品接线 · c635→c645；demo 已有）

| 键 | 行为 | Change |
|---|---|---|
| Ctrl+D | filter → default | **c635** |
| Ctrl+T/U/L/A | filter **toggle** ↔ default（no-tools / user / labeled / all） | **c635** |
| Ctrl+O | filter cycle forward（树开优先；关树仍为工具视口） | **c635** |
| Ctrl/Alt+←→ | fold / 分支跳转（转发包；裸 ←→ 仍翻页） | **c640** |
| Shift+L / T | annotation / 时间戳 | **future**（本波不做） |
| Shift+F | **fork** | **c645** |
| Enter | `travel_session_tree`（已 **c615**） | — |

## MUST（编辑器槽）

| 键 | 行为 |
|---|---|
| Esc（选择器打开时） | 关闭选择器，还原 editor |
| `/` 补全 | CompletionSource；产品命令含 `/exit` `/model` |
| `!` / `!!` 前缀 | bash 边框 + idle Enter → `execute_bash`（**c492**） |
| Ctrl+G | 外部编辑器：TTY 真 `$VISUAL`/`$EDITOR`（**c650**）；harness / 非 TTY 仍 stub |

## 明确不做（键位）

- Settings / Plate 槽的运行时配置编辑（配置走 YAML+JSON Schema）。
- computer-use 专用键位（本波延后）。

## 规则

1. 全局键经 `InputListener` **先于** Editor 焦点消费（c455）。
2. **MUST NOT** 让 Ctrl+C 泄漏进 Editor 变成字面 `c`。
3. footer **默认不**罗列完整快捷键墙；细节 `/help`（若有）。
4. UI 旁注快捷键 MUST 用括号包裹完整和弦（如 `(Ctrl+T)`、`(Alt+E)`），**MUST NOT** 使用 `^T` 缩写；提示色 `{colors.muted}`。
