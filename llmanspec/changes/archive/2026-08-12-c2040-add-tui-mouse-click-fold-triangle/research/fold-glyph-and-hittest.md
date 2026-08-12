# Research: 折叠字形与 Hit-test

> Change: `c2040-add-tui-mouse-click-fold-triangle`
> **架构级调研**（选区 oneof / starline / Alt-hold）已迁至 [`../../research/`](../../research/)。

## 今日字形

`src/app/tui/widgets/glyphs.rs`：

| | Unicode | Ascii |
|---|---|---|
| fold（收起态可展开） | `▶` | `>` |
| unfold（展开态可收起） | `▼` | `v` |

`c1760` 已拍：用 `▶/▼`，**不做** `(+)/(-)`。本草案只允许同族微调。

## 字形候选（目视后拍）

| 对 | 观感 | 风险 |
|---|---|---|
| 保持 `▶`/`▼` | 零迁移 | 用户要「更好看」未满足 |
| `▸`/`▾`（小三角） | 更轻、常用于树 | 部分字体窄/糊 |
| `▷`/`▽` | 空心，更「可点」 | 个别终端缺字元 → 宽/豆腐 |
| `❯`/`﹀` 等 | 易与 user glyph `❯` 撞 | **否** |

**约束**：`visible_width` == 1；Ascii 保持单字符；缺字时 env `XYLITOL_TUI_GLYPH_SET=ascii`。

## Hit-test

`c1760` 预留：

1. render 记 `id → [line_start, line_end)`
2. click y → id → toggle
3. 非标记区保持今日行为

本草案收紧：

- **优先**标记列（x 在标记可视宽内）才 toggle，降低误触。
- 整行可点可作为增强（配置或第二波）。
- 表与 `ScrollbackPaintCache` generation 绑定；fold 变更 bump generation。

## 与差分引擎

点击只改 UI 态 → `request_render`；差分仍按行。高度变化时 viewport 贴底策略保持现有「跟底」心智。**MUST NOT** 为 hit 表改 `previous_lines` 语义（那是 `c1370`）。

## 依赖

无 `c2020` 无法收真实 Mouse；harness 可先注入合成 `InputEvent::Mouse`（地基落地后）。
