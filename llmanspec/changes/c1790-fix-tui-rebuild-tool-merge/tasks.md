# Tasks: c1790-fix-tui-rebuild-tool-merge

## 1. Specs

- [x] 1.1 收紧 live `app-tui-transcript` `att12`：重建 Tool 与 live End 后单块幂等；同 toolCallId → 恰好一条 `UiEntry::Tool`
- [x] 1.2 对应 `*.feature` 场景（`@req:att12`）+ `llman sdd validate … --strict --no-check`
- [x] 1.3 `llman sdd change start c1790-fix-tui-rebuild-tool-merge` → branch `sdd/c1790-…`（Stage: full）

## 2. 共享投影

- [ ] 2.1 抽出/复用 live End 语义的「toolResult → 合入已有 Tool」API（bridge）
- [ ] 2.2 `rebuild_scrollback_from_travel` / 路径投影改走共享逻辑；toolResult MUST NOT 在可配时时另起 entry-uuid Tool

## 3. 验证

- [ ] 3.1 单测：assistant(toolCall)+toolResult fixture → 恰好 1× Tool 且 preview+output+done
- [ ] 3.2 orphan toolResult（无匹配 call）仍可投影为单行
- [ ] 3.3 相关 `cargo test` + validate --strict
