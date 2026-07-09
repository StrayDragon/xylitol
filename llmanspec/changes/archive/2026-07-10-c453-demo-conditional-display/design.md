# Design — c453-demo-conditional-display

## 应用层状态机（不进包）

| 块 | 折叠态 | 展开态 | 键 |
|---|---|---|---|
| Thinking | 一行摘要 + `(Ctrl+T)` | 全文 | Ctrl+T（全局切换） |
| Tool / Diff | 摘要行 + `(Alt+E)` | 详情 / Diff 正文 | Alt+E（tool+diff 一起） |

流式 thinking：写入期间保持展开；`StreamFinish(Thinking)` 后默认折叠。

与 **Ctrl+O**（c466 视口 max-height）正交：Alt+E = 块有无详情；Ctrl+O = 详情可见时的视口/全文。
