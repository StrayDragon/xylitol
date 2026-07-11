# DESIGN playground（人类 · 可交互设计图）

对照 `../DESIGN.md` + `design/*.md`，在浏览器里审色、典型态与动态反馈，再决定改 token 或落地 `xylitol-tui`。

**Agent 默认忽略本目录**（见 [`../AGENTS.md`](../AGENTS.md)），除非人类指定或粘贴片段。

## 打开

```bash
xdg-open src/app/tui/design/playground/index.html
# 或
cd src/app/tui/design/playground && python -m http.server 8765
# http://127.0.0.1:8765/
```

## 改 DESIGN 后同步（P1）

```bash
python3 src/app/tui/design/playground/sync_tokens.py
```

生成物（勿手改）：`tokens.css`、`tokens.js`。禁止为迁就预览去改 `xylitol-tui` 运行时默认主题。

## 视图 / 跳转

| 输入 | 作用 |
|---|---|
| **左侧 tab 点击** | 选中该槽并 **滚动跳转到对应面板**（平铺/专注均生效） |
| 平铺 / 专注 · `t` | 全槽一页 vs 只看当前槽 |
| `1`–`7` · `←→` | 选槽并跳转 |
| 深链 | `?slot=tool&mode=focus`（会写入 URL） |

## 槽内动态

| 槽 | 可切换 |
|---|---|
| Tool | 三态 · 流式 thinking · 展开/视口 · Diff 块「摘要 tint / 正文不套 tool-bg」对照 |
| Diff | unified / SBS（SBS 无行底）· 词级 |
| Markdown | 四格典型态平铺 · 假流式逐行 |
| Layout | busy↔idle · steer 提示 · 双 Esc 会话树替换 editor |
| Chrome | 一处 accent ✓ vs 多处 ✗ · user / fg |
| Overlay | 确认框显隐（短过渡）· Esc/N/Y |

目标：改 DESIGN 或 MUST 时，先在此获得最快视觉反馈，再落地包组件 / demo。
