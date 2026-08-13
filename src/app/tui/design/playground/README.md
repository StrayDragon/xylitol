# DESIGN playground（产品静图 SSOT）

**归属**：仅服务产品面 `src/app/tui`（DESIGN / layout / chrome）。**不是** `packages/xylitol-tui` 的 `agent_demo`；包交互演示与本页 **允许差异**。

| 页 | 角色 |
|---|---|
| [`index.html`](./index.html) | **唯一**产品静图 SSOT：形状 / 色板 / 整壳固定态 ↔ DESIGN；**UiEntry 默认 rail** |

重制说明：[`docs/roadmaps/TUI重制.md`](../../../../../docs/roadmaps/TUI重制.md)（M1 产品已兑现）。

本仓 **不**在包内维护第二份 design HTML。**Agent 默认忽略本目录**（见 [`../AGENTS.md`](../AGENTS.md)），除非人类指定。

信息面用词：[`docs/architecture/TUI信息面与chrome词汇.md`](../../../../../docs/architecture/TUI信息面与chrome词汇.md)。

## 闸门

```bash
just check-tui-tokens
python3 scripts/check_tui_design_playground.py --check   # index.html + fixtures
```

## 打开

```bash
just open-design-playground
```

## 改 DESIGN 后同步

```bash
just sync-tui-tokens
just check-tui-tokens
```

生成物（勿手改）：`tokens.css`、`tokens.js`。

## UiEntry rail（SSOT 默认）

- 左边轨 = **CSS border**（静图）或 **窄 bg 条**（产品 / `paint_left_rail_line`）
- **禁止** ASCII `|`；**禁止**默认整行蓝绿洗底
- tool / bash / diff：轨 + gutter；user / assistant / **thinking**：flush
- 工具类条目上方 ≥1 空行

## SSOT 槽（index）

Full shell · Layout · Keybindings · Models · Pending · Chrome toast · Mcp · Tree · Resume · Compaction · Activity fold（可点） · Tool · Diff · Markdown · Palette · Widgets · Atoms · Ask
