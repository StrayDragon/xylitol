# design — c481 app-tui-editor-history

## 问题

产品 Editor 有 ↑/↓ 历史导航，但发送路径不写 history，行为等同未实现。

## 决策

| 发送路径 | 写 history |
|---|---|
| idle Enter → submit | 是（trim 非空） |
| busy Enter → steer | 是 |
| Alt+Enter → follow-up | 是 |
| Alt+Up restore / slash | 否（restore 不新增；slash 命令可不记或只记非 `/`——MVP：**slash 不写入**） |

实现：`UiRoot::remember_editor_send(text)` → `editor.add_to_history`；host 三处入队/提交前调用。

对齐 pi：`interactive-mode` 在 prompt/steer/followUp 时 `editor.addToHistory?.(text)`。
