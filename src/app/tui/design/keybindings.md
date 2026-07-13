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
> **c480 MVP** 先落地「全局 / 输入」表；树开扩展键仅在 **活树** change 后启用（c491 stub 只开/关/Enter travel）。

已决议产品键位（实现：c480 / InputListener c455）。活实验场：`just demo-tui`。

## MUST — c480 MVP（全局 / 输入）

| 键 | 行为 |
|---|---|
| Esc | 流中：**abort**（清 steer，**留** follow_up 供 restore） |
| Ctrl+C | 编辑器非空：**清空**；已空：**退出** TUI |
| Enter（idle） | 提交用户消息 |
| Enter（流中） | **steer** |
| Alt+Enter | **follow-up**（排队到本轮结束后） |
| Alt+Up | 将已排队 steer/follow-up **还原进 editor** 并清空两侧队列 |
| ↑ / ↓（editor） | 在首/末可视行且（空草稿或已在浏览）时：**发送历史**召回（c481；包 ed05） |
| 双 Esc | 打开 **c491 stub** 会话树（假树；Esc 关；Enter `travel → id`） |
| `/exit` | 退出并 restore（与垂直切片一致） |
| `/model` | 切换/选择模型（MVP；实现可极简列表） |

## MUST — 树关时（有内容时；可与 c480 同批或紧随）

| 键 | 行为 |
|---|---|
| Ctrl+O | 工具详情视口折叠/全文 |
| Ctrl+T | thinking 展开/折叠 |
| Alt+E | tool/diff **块**展开/折叠 |

## 后置 — 树开（**勿**在 c491 stub 上实现）

| 键 | 行为 |
|---|---|
| Ctrl+O/T/D/U/L/A | filter 循环 |
| Ctrl/Alt+←→ | fold / 分支跳转 |
| Shift+L / T / F | annotation / 时间戳 / **fork** |
| Enter | travel（demo **c600**：user → 父 leaf + 填 input；非 user → leaf=id） |

## MUST（编辑器槽）

| 键 | 行为 |
|---|---|
| Esc（选择器打开时） | 关闭选择器，还原 editor |
| `/` 补全 | CompletionSource（包注册表）；产品 MVP 命令见上 |
| `!` / `!!` 前缀 | bash 边框 + idle Enter → `execute_bash`（**c492**） |
| Ctrl+G | 外部编辑器 stub（**c492**；真 `$EDITOR` 后置） |

## 规则

1. 全局键经 `InputListener` **先于** Editor 焦点消费（c455）。
2. **MUST NOT** 让 Ctrl+C 泄漏进 Editor 变成字面 `c`。
3. footer **默认不**罗列完整快捷键墙；细节 `/help`。
4. UI 旁注快捷键 MUST 用括号包裹完整和弦（如 `(Ctrl+T)`、`(Alt+E)`），**MUST NOT** 使用 `^T` 缩写；提示色 `{colors.muted}`。
