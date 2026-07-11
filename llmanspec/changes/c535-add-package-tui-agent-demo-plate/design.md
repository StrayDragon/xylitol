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
| A（先行） | 加厚 Markdown showcase；`/md` + palette「markdown」入口 | 可在本提案 apply 前合入 |
| B（本变更） | plate 目录化、预制 prompt 表、瘦 seed、收 footer | apply 本 change |

## 预制 prompt 表（初稿）

| plate id | 触发 | 行为 |
|---|---|---|
| `md-full` | `/md` 或 plate | 推全语法 assistant（c530） |
| `stream-rust` 等 | plate / 关键词 | `queue_simulated_turn` 定点语言 |
| `diff-sbs` | plate | 现有 SBS + unified 推送 |
| `tool-tints` | plate | pending/success/error 三态块 |
| `tree` | plate | 打开会话树槽 |
| `help-keys` | plate / `/help` | 键位说明进 transcript，不占 footer |

## 非目标

- 产品 `protocol::Command` 真接线
- 把 plate 做成居中 overlay 仪表盘（仍替换 editor 槽，对齐 DESIGN）
