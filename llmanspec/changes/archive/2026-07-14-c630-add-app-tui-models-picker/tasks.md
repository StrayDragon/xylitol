# Tasks — c630-add-app-tui-models-picker

- [x] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c630-add-app-tui-models-picker --no-interactive`
- [x] 2. `PendingSlash`：无参 `/model` → `OpenModels`；有参仍 `SetModel`；停用 cycle 主路径
- [x] 3. `EditorSlot::Models` + `UiRoot` 挂载 `SelectList`（主题对齐 LayoutTheme）；过滤键入；Esc 关槽
- [x] 4. `effects`：`OpenModels` → `GetAvailableModels` → mount；选定 → `SetModel` + footer；idle-only
- [x] 5. harness：开槽 / 过滤选定 / Esc 不变 / 无参不再 CycleModel；`/model <id>` 直选
- [x] 6. `just fmt` + 相关 test / clippy 绿
