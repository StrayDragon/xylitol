# DESIGN playground（静态设计图）

**本目录 = 产品视觉的浏览器静态图**：固定状态对照 [`../../DESIGN.md`](../../DESIGN.md) + 上级 `design/*.md`。

三层职责：

| 层 | 命令 / 路径 | 角色 |
|---|---|---|
| **静态设计图** | 本目录 HTML | 形状 / 色板 / 整壳定稿对照 |
| **动态 playground** | `just demo-tui` | 可交互试键位与组件 |
| **生产** | `src/app/tui/` | 真 host 接线 |

本仓 **不**在包内维护第二份 design HTML。**Agent 默认忽略本目录**（见 [`../AGENTS.md`](../AGENTS.md)），除非人类指定或粘贴片段。

静图 **不**标注实现分层（无「包」标签），**不**在顶栏堆快捷键百科。

## 闸门（防 DESIGN 漂移）

```bash
just check-tui-tokens
python3 scripts/check_tui_design_playground.py --check
# 二者均经 just qa → check-scripts / check-tui-tokens
```

夹具：[`../fixtures/`](../fixtures/)（L2）。改形状先改 fixture / DESIGN，再改 HTML。

## 打开

```bash
just open-design-playground
# 或
xdg-open src/app/tui/design/playground/index.html
```

深链示例：`?slot=models&mode=focus&scheme=light` · `?slot=session-resume`

## 改 DESIGN 后同步

```bash
just sync-tui-tokens
just check-tui-tokens   # tokens.css/js + Palette ≡ DESIGN.md
```

生成物（勿手改）：`tokens.css`、`tokens.js`。

## 视图

| 输入 | 作用 |
|---|---|
| 左侧 tab | 选槽 |
| 平铺 / 专注 · `t` | 全槽 vs 当前槽 |
| Dark / Light · `d` / `l` | `colors` vs `colors_light` |
| 字母 / 数字 / `←→` | 跳槽（可选；顶栏不展示提示墙） |

## 槽

| 槽 | 说明 |
|---|---|
| Full shell · Layout · Keybindings · Models · Pending / NextTurn · **Mcp** (`/mcp` + 固定 cue，c1210 已落地) · Tree power · Resume · Compaction | 产品整页 / 下一波形状 |

| Tool · Diff · Markdown · Overlay · Palette | 呈现积木 |
| Widgets · Atoms · Ask | 其它固定形状槽（与实现分层无关） |

深链：`?slot=mcp-cue`（as-built `/mcp` 行文 + `mcp pending (see /mcp)`；见 [`../mcp-input-cue.md`](../mcp-input-cue.md)）。
