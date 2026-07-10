# Keybindings

已决议产品键位（实现：c480 / InputListener c455）。包 demo 应提前对齐以便验证。

## MUST（全局 / 输入）

| 键 | 行为 |
|---|---|
| Esc | 流中：**abort** 当前模型/工具流 |
| Ctrl+C | 编辑器非空：**清空**；已空：**退出** TUI |
| Enter（流中） | **steer**（插入引导，不打断当前轮的队列语义见 c461） |
| Alt+Enter | **follow-up**（排队到本轮结束后） |
| 双 Esc | 打开 **会话树**（后置；见 [`session-tree.md`](./session-tree.md)） |

## MUST（编辑器槽）

| 键 | 行为 |
|---|---|
| Esc（选择器打开时） | 关闭选择器，还原 editor |
| Ctrl+P 等 | 打开命令/设置（替换 editor 槽） |

## 规则

1. 全局键经 `InputListener` **先于** Editor 焦点消费（c455）。
2. **MUST NOT** 让 Ctrl+C 泄漏进 Editor 变成字面 `c`。
3. footer **默认不**罗列完整快捷键墙；细节 `/help`。
