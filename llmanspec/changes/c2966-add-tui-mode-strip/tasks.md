# Tasks

- [ ] 1. **specs**（spec）：`app-tui-fixed-zone` 新增 `mode-strip-single-line` 规则并把固定区顺序改为 queue → todo → ModeStrip → toast → status → editor；`app-tui-bridge` 记录 `planModeChanged` 映射。测：`llman-sdd validate --all --strict`。
- [ ] 2. **组件**（app-tui）：新增 `widgets/mode_strip.rs`（`ModeStripState` + 单行渲染、右截断、ascii 回退）；`UiModel` 增 `mode_strip`。测：单行/隐藏/截断/ascii 单测。
- [ ] 3. **布局**（app-tui）：`layout/root` 渲染 ModeStrip 并纳入 `estimate_dock_rows` 与 `reserved_lower_fixed_zone`；空态 0 行。测：footprint 与顺序单测。
- [ ] 4. **事件接线**（app-tui）：`planModeChanged` 更新 `UiModel.mode_strip`；`execute` 无 plan 时隐藏。测：host 事件单测 + snapshot。
- [ ] 5. **文档**（docs）：`DESIGN.md` / 端 AGENTS 记录新固定区槽位。测：`just qa-doc-check`。
- [ ] 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
