# Design — c449-split-app-tui-design-docs

## Approach

纯文档拆分：主 `DESIGN.md` 保留全局 tokens + 跨组件硬规则；组件级 MUST 下沉到 `design/<name>.md`。实现 change（c451 Diff、c475 chrome、c480 input）只引用子文档，不在口头补规则。

## File map

| 文件 | 职责 |
|---|---|
| `DESIGN.md` | 索引 + 全局 tokens + Layout/Colors/Typography/Elevation |
| `design/transcript.md` | 消息呈现 |
| `design/expandable.md` | thinking/tool 可展开 |
| `design/status.md` | busy 一行 |
| `design/editor.md` | 操作区边框与槽替换 |
| `design/footer.md` | 一行 dim |
| `design/overlay.md` | 短确认 |
| `design/diff-block.md` | unified / side-by-side / word-level / CJK |
| `design/glyphs.md` | unicode/ascii 档 |
| `design/theme-tokens.md` | 语义 → SGR |
| `design/keybindings.md` | 已决议键位 |
| `design/markdown.md` | 复制友好 markdown |
| `design/errors.md` | 错误一行 |
| `design/session-tree.md` 等 | 后置能力草稿 |

## Non-goals

不改包 API；不改 `src/app/tui` 运行时代码（除 AGENTS 指针）。
