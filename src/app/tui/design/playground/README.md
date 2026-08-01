# DESIGN playground（静图）+ remaster prototype

| 页 | 角色 |
|---|---|
| [`index.html`](./index.html) | **SSOT 静图**：形状 / 色板 / 整壳固定态 ↔ DESIGN |
| [`uientry-remaster.html`](./uientry-remaster.html) | **独立 prototype**：`UiEntry` 风格皮肤（主方案 **rail**）；不进 fixture |

重制短链路：[`docs/roadmaps/TUI重制.md`](../../../../../docs/roadmaps/TUI重制.md)。

本仓 **不**在包内维护第二份 design HTML。**Agent 默认忽略本目录**（见 [`../AGENTS.md`](../AGENTS.md)），除非人类指定。

## 闸门

```bash
just check-tui-tokens
python3 scripts/check_tui_design_playground.py --check   # 只卡 index.html + fixtures
```

## 打开

```bash
just open-design-playground          # SSOT
just open-uientry-remaster           # rail 皮肤 prototype
```

## 改 DESIGN 后同步

```bash
just sync-tui-tokens
just check-tui-tokens
```

生成物（勿手改）：`tokens.css`、`tokens.js`（两页共用）。

## rail 皮肤（当前主方案）

- 左边轨 = **CSS border**（静图）或 **窄 bg 条**（`just demo-tui-rail` / `/entry-style rail`）
- **禁止** ASCII `|`；**禁止**默认整行蓝绿洗底（wash 仅作对照）
- 每个 `UiEntry` 上方 ≥1 空行；多行流式共用一条轨
- 场景：`happy turn` · `stream` · `errors` · `compaction` · `skills+mcp` · `! bash + queue` · `kitchen`
- 深链：`?scene=bang-queue&style=rail`



## SSOT 槽（index）

Full shell · Layout · Keybindings · Models · Pending · Chrome toast · Mcp · Tree · Resume · Compaction · Tool · Diff · Markdown · Palette · Widgets · Atoms · Ask
