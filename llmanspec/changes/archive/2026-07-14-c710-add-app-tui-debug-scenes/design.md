# Design — c710-add-app-tui-debug-scenes

## 决策

| 点 | 选择 | 理由 |
|---|---|---|
| 入口 | idle slash `/debug` / `/debug:<id>` / `/debug <id>` | 对齐 DESIGN「调试走 /debug」；不占底栏 |
| Session | **新建** `debug-{scene}-{8hex}` 并 switch | 不污染用户当前会话 |
| 协议 | Driver-only（同 session_tree） | 不扩 `protocol::Command` |
| Fake | catalog 有则 `select_model`；无则只种子树 + note | 避免热改 registry / 写坏用户 config |
| 场景 v1 | `tree-branch`（多轮）、`tree-labeled`（+ Label） | 覆盖 Search/Help/filter/label 手测 |

## 验证

- harness：解析 + load 调用
- 人类：见 proposal
