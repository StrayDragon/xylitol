# Design — c1005-update-app-tui-session-slash-rename

## pi 行为调研（对照源）

源：`../pi/packages/coding-agent` — `src/core/slash-commands.ts` + `src/modes/interactive/interactive-mode.ts`。

| pi 命令 | 行为摘要 | xylitol 决议（本 change） |
|---|---|---|
| `/tree` | `showTreeSelector()`：开树；travel 前可问「是否 summarize branch」（LLM） | 改名 **`/session-tree`**；开树同双 Esc。**不做** travel 摘要（PI_DELTAS A01） |
| `/fork` | `showUserMessageSelector()`：选历史 **user** 消息 → `runtimeHost.fork` → 新会话并把选中文本填回 editor | 改名 **`/session-fork`**；语义保持产品 **leaf fork+switch**（同 Shift+F / A02），**不开** user 选择器 |
| 旧名 | 即为产品名 | 旧名 **`/tree` `/fork` 无效**（unknown slash） |

## 实现要点

- 仅 rename 解析 token + SlashCommandSource + harness/文档针；不改 Driver fork/tree API。
- unknown 提示串同步新名。

## 非目标

- 对齐 pi 的 user-message fork UX；Batch A/B 新命令。
