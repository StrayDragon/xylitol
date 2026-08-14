# DESIGN playground（双轨旧静图）

**归属**：未迁槽的产品静图。**意图 SSOT 已迁** [`../../designing/`](../../designing/)（`just open-designing`）。本页不是唯一视觉 SSOT。

| 页 | 角色 |
|---|---|
| [`../../designing/`](../../designing/) | 模块化意图 + 固定态（人类浏览器；Agent 默认读模块） |
| [`index.html`](./index.html) | 双轨旧巨石；闸对其它槽仍解析；activity-fold 闸已走 YAML states |

重制说明：[`docs/roadmaps/TUI重制.md`](../../../../../docs/roadmaps/TUI重制.md)（M1 产品已兑现）。

本仓 **不**在包内维护第二份 design HTML。**Agent 默认忽略本目录 HTML**（见 [`../AGENTS.md`](../AGENTS.md) / [`../../designing/AGENTS.md`](../../designing/AGENTS.md)）。

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
