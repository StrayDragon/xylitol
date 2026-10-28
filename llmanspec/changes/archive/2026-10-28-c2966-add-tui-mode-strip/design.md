# Design

## Component

```text
ModeStripState { glyph: &'static str, label: String }
```

- `glyph` comes from the glyph set (`◆` unicode / `*` ascii); `label` is a short state such as `plan`, `execute · p1`.
- Render one line, left-aligned, dim/accent by mode; no border, no background, no wrapping.
- If the label exceeds the available width, truncate on the right (same rule as footer), never grow to two lines.
- Empty state renders zero rows.

## Data flow

- `UiModel` gains `mode_strip: Option<ModeStripState>`.
- `planModeChanged { mode, plan_id }` maps to:
  - `plan` → `ModeStripState { glyph: "◆", label: "plan" }`
  - `execute` with a plan → `ModeStripState { glyph: "◆", label: "execute · <plan_id>" }`
  - `execute` without a plan → `None` (normal mode is not a special mode).
- The host applies the event to `UiModel` and calls `set_mode_strip` on `UiRoot`.

## Layout

Fixed-zone order becomes:

```text
queue strip → todo bar → ModeStrip → toast notice → status → editor
```

- `ModeStrip` sits below the todo bar so plan mode remains visible while the todo list is being built.
- It is not part of scrollback, not a `ScrollNotice`, not a client notice and not a toast.
- `estimate_dock_rows` and `reserved_lower_fixed_zone` add exactly one row when the strip is visible.

## Rendering

- A small widget module owns the line rendering; `layout/root` only places it.
- Use the existing theme tokens: `muted` for the glyph/label, `accent` for the active plan glyph if the theme already exposes it. No new palette token is required.
- ASCII glyph set falls back to `*`.

## Specs

- `app-tui-fixed-zone`: new `mode-strip-single-line` rule; amend the fixed-zone order rule to include the strip.
- `app-tui-bridge`: `planModeChanged` → `UiModel.mode_strip`.

## Tests

- Widget unit: single line, hidden when empty, truncation, ASCII fallback.
- Layout unit: footprint adds one row only when visible; order is queue → todo → strip → toast → status → editor.
- Host unit: `planModeChanged` updates the strip; `execute` without plan hides it.
- Snapshot: plan mode frame contains exactly one mode line.
- `just qa` and `llman-sdd validate --all --strict`.

## Task breakdown

- 1. **specs**（spec）：`app-tui-fixed-zone` 新增 `mode-strip-single-line` 规则并把固定区顺序改为 queue → todo → ModeStrip → toast → status → editor；`app-tui-bridge` 记录 `planModeChanged` 映射。测：`llman-sdd validate --all --strict`。
- 2. **组件**（app-tui）：新增 `widgets/mode_strip.rs`（`ModeStripState` + 单行渲染、右截断、ascii 回退）；`UiModel` 增 `mode_strip`。测：单行/隐藏/截断/ascii 单测。
- 3. **布局**（app-tui）：`layout/root` 渲染 ModeStrip 并纳入 `estimate_dock_rows` 与 `reserved_lower_fixed_zone`；空态 0 行。测：footprint 与顺序单测。
- 4. **事件接线**（app-tui）：`planModeChanged` 更新 `UiModel.mode_strip`；`execute` 无 plan 时隐藏。测：host 事件单测 + snapshot。
- 5. **文档**（docs）：`DESIGN.md` / 端 AGENTS 记录新固定区槽位。测：`just qa-doc-check`。
- 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
