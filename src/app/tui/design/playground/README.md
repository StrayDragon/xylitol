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
# 验收：改 colors.accent → 跑脚本 → 刷新 HTML，色应变
python3 src/app/tui/design/playground/sync_tokens.py
```

生成物（勿手改）：`tokens.css`、`tokens.js`。禁止为迁就预览去改 `xylitol-tui` 运行时默认主题。

## 视图

| 输入 | 作用 |
|---|---|
| 平铺 / 专注 · `t` | 全槽一页 vs 只看当前槽 |
| 左栏 · `1`–`7` · `←→` | 选槽 |
| 深链 | `?slot=tool&mode=focus` |

## 槽内动态（设计图典型态）

| 槽 | 可切换 |
|---|---|
| Tool | pending/success/error · 展开 (Alt+E) · 视口全文 (Ctrl+O) · thinking (Ctrl+T / 点摘要) |
| Diff | 词级开/关 · 仅摘要行 tint |
| Markdown | 正文 / 代码 / 链接 / 引用 |
| Layout | busy↔idle（status 0 行）· editor↔选择器槽 · spinner 动画 |
| Chrome | user / muted / accent / fg 语义 |
| Overlay | 显示/隐藏 · Esc/N/Y |

目标：改 DESIGN 或 MUST 时，先在此获得最快视觉反馈，再落地包组件 / demo。
