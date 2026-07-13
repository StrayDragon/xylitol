# DESIGN playground（产品 · 浏览器静图）

**本目录 = 产品视觉浏览器预览**，对照 [`../../DESIGN.md`](../../DESIGN.md) + 上级 `design/*.md`。

**活实验场**（终端、可交互形状）是：

```bash
just demo-tui   # packages/xylitol-tui/examples/agent_demo.rs
```

`agent_demo` 即产品 TUI 的快速 playground：先在此试，再接线 `src/app/tui`。本仓 **不**在包内维护第二份 design HTML。

**Agent 默认忽略本目录**（见 [`../AGENTS.md`](../AGENTS.md)），除非人类指定或粘贴片段。

## 打开

```bash
xdg-open src/app/tui/design/playground/index.html
# 或
cd src/app/tui/design/playground && python -m http.server 8765
# http://127.0.0.1:8765/?slot=shell
```

## 改 DESIGN 后同步

```bash
python3 src/app/tui/design/playground/sync_tokens.py
# 或
just sync-tui-tokens
just check-tui-tokens   # tokens.css/js + Palette ≡ DESIGN.md
```

打开浏览器静图（Linux）：

```bash
just open-design-playground
# ≡ xdg-open src/app/tui/design/playground/index.html
```

生成物（勿手改）：`tokens.css`、`tokens.js`。

## 视图 / 跳转

| 输入 | 作用 |
|---|---|
| **左侧 tab** | 选槽并跳转 |
| 平铺 / 专注 · `t` | 全槽 vs 当前槽 |
| Dark / Light · `d` / `l` | `colors` vs `colors_light` |
| `f` | **Full shell**（c475+c480 整壳） |
| `c` | **Layout 壳**（c475） |
| `k` | **Keybindings**（c480 MVP） |
| `1`–`5` · `←→` | Layout / Tool / Diff / Markdown / Overlay |
| `8` `9` `0` | 弱对照：Widgets / Atoms / Ask（细节以 `just demo-tui` 为准） |
| 深链 | `?slot=shell&mode=focus&scheme=light` |

## 槽优先级

| 优先级 | 槽 | 说明 |
|---|---|---|
| **主** | Full shell · Layout · Keybindings | 产品整页合成与语义 |
| **产品积木** | Tool · Diff · Markdown · Overlay | 仍属产品呈现 MUST |
| **弱对照** | Widgets · Atoms · Ask | 折行/焦点以 demo 为准 |
