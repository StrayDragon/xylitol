# Design — c463-revise-app-tui-transcript-scope

## Decisions

1. **Keep capability id** — `app-tui-transcript` 目录保留，避免大范围 rename；语义改为 live scrollback。
2. **Remove att2** — 产品不再 MUST 实现 Expandable 消息栈；demo（c453/c462）可继续有折叠原型，但不升格为产品硬合约。
3. **Add att6** — 明文禁止 Codex 式 TranscriptView；分支 UX 指向会话树。
4. **att4 Diff header-only** — 与已验证的 `design/expandable.md` 对齐，避免归档后仍要求「Diff 正文也套 tool bg」。

## Alternatives rejected

- 删除整个 capability（破坏 `app-tui` 索引与历史 delta 链）。
- 把 live 行合约并进 `app-tui-bridge`（bridge 管事件翻译，不该吞呈现约束）。
