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

## 改 DESIGN 后同步（c555 / adp1）

```bash
python3 src/app/tui/design/playground/sync_tokens.py
```

生成物（勿手改）：`tokens.css`、`tokens.js`。SSOT 是 `DESIGN.md` frontmatter，不是本目录手改色值。禁止为迁就预览去改 `xylitol-tui` 运行时默认主题。

Markdown 槽示意 MUST 对齐 [`../markdown.md`](../markdown.md)：无 `#` 标题前缀、链接 `text (url)`、粗体/斜体无可见 `**`/`*`（可用「（加粗）」标注）。

## 视图 / 跳转

| 输入 | 作用 |
|---|---|
| **左侧 tab 点击** | 选中该槽并 **滚动跳转到对应面板**（平铺/专注均生效） |
| 平铺 / 专注 · `t` | 全槽一页 vs 只看当前槽 |
| `1`–`9` · `0` · `←→` | 选槽并跳转（`0` = Ask） |
| 深链 | `?slot=widgets&mode=focus`（会写入 URL） |

## 槽内动态

| 槽 | 可切换 |
|---|---|
| Tool | 三态 · 流式 thinking · 展开/视口 · Diff 块「摘要 tint / 正文不套 tool-bg」对照 |
| Diff | unified / SBS（SBS 无行底）· 词级 |
| Markdown | 强调样式（已落地 B 色+字重）· 嵌套列表悬挂缩进 · 假流式逐行 |
| Layout | busy↔idle · steer 提示 · 双 Esc 会话树替换 editor |
| Chrome | 一处 accent ✓ vs 多处 ✗ · user / fg |
| Overlay | 确认框 shown+focus / unfocus / hidden · Esc/N/Y（引擎 OverlayHandle；demo 用内联槽） |
| Widgets | Select/Settings 空态 · Input/Loader 窄宽（对照 `narrow-clamp`） |
| Atoms | TruncatedText · Panel · CancellableLoader（对照同名 plate） |
| Ask | ChoicePrompt：单选无 ●/○ · 多选 [x] · 多题混搭 Tabs（c565 · `ask-*`） |

目标：改 DESIGN 或 MUST 时，先在此获得最快视觉反馈，再落地包组件 / demo。

## 运行时参考（库用户）

浏览器预览只看色与层次；**行为与折行**以包 + demo 为准：

```bash
just demo-tui
# Ctrl+P → md-list-wrap   嵌套列表 / 悬挂缩进（单次折行）
# Ctrl+P → narrow-clamp          Settings 搜索空态 · Input/Loader 窄宽
# Ctrl+P → truncated-text        TruncatedText 单行省略
# Ctrl+P → panel                 Panel padding+bg
# Ctrl+P → cancellable-loader    Esc → on_abort（完整 braille 转圈）
# Ctrl+P → ask-single|ask-multi|ask-tabs   ChoicePrompt（c565）
# /md                            完整 Markdown grammar stub
```
