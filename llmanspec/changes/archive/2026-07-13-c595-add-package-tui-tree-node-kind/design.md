# Design — c595 TreeNode.kind

## 决议

| 选项 | 取舍 |
|---|---|
| 包内 `enum TreeNodeKind` | **否** — 会话 role 属产品；库用 `Option<String>` |
| host 预烘焙 `user: …` 进 label | **否** — 迁到 `kind` + `kind_prefix` 主题 |
| 多 badge 列表 `tags: Vec` | **延后** — 本切片只要单一 kind 前缀（对齐 pi 行首 role） |

## 渲染序

```
›  ├─ [annotation]? [annotation_at]? <kind_prefix(kind)> <label>
```

- `kind` 缺省：不画前缀（纯 `label`，兼容旧调用方）。
- 主题默认：`user`→accent 系、`assistant`→success/on-surface、`tool`→muted；未知 kind 用 dim `[kind]: `。

## Demo 约定（非包合约）

| kind | 含义 |
|---|---|
| `user` | 用户回合 |
| `assistant` | 助手回合 |
| `tool` | 工具调用摘要 |
| （其它） | host 自定；默认主题 dim |

## SSOT

视觉 MUST 只在 `src/app/tui/design/session-tree.md` + playground 对照；包不另起 design 树。
