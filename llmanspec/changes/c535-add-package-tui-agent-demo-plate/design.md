# Design — c535-add-package-tui-agent-demo-plate

## 目标形态

```
默认 seed（短）
  └─ System: Ctrl+P plate · /md · /help …
用户 Ctrl+P
  └─ Command plate（替换 editor 槽）
        demo: markdown-full
        demo: stream-rust | stream-python | …
        demo: diff-sbs | tool-tints | tree | bash
        demo: slim-seed / reset
选中 → 注入预制 prompt 或直接 ScriptEvent 序列
```

## 与 A 步关系

| 步 | 内容 | 状态 |
|---|---|---|
| A（先行） | 加厚 Markdown showcase；`/md` + palette「markdown」入口 | 已合入；现为 `md-full` 打字机 stub |
| B（本变更） | plate 目录化、预制 prompt 表、瘦 chrome、收 footer | apply 中 |

## 预制 prompt 表

| plate id | 触发 | 行为 |
|---|---|---|
| `md-full` | `/md` 或 plate | 打字机流式推全语法 stub（`markdown_grammar_stub`） |
| `stream-rust` 等 | plate | `commit_user_turn` 定点语言 |
| `diff-sbs` | plate / `/diff` | unified + SBS + display_diff |
| `tool-tints` | plate | thinking + success/error/long bash |
| `tree` | plate | 打开会话树槽 |
| `help-keys` | plate / `/help` | 键位说明进 transcript，不占 footer |
| `tests` / `compact` | plate | 假 cargo test / compaction 注记 |

默认 seed 保留 **compact kit**（短帮助 + thinking/tools/diff 各一份）便于开箱验收折叠/着色；全语法 Markdown 只走 plate 流式。

## 非目标

- 产品 `protocol::Command` 真接线
- 把 plate 做成居中 overlay 仪表盘（仍替换 editor 槽，对齐 DESIGN）
