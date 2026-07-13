# Tasks — c494-refactor-app-tui-editor-slot

- [x] 1. 抽出 `effects::drain_pending`；`mod.rs` 与 `harness` 共用；确认 H1–H9 基线绿
- [x] 2. 抽出 `commands`（slash/bang）；行为对齐 c480/c492
- [x] 3. 落地 `layout/slots.rs`：`EditorSlot`；迁移 c491 假树进 `Tree` 变体
- [x] 4. （MAY）接线 Plate / Settings / Choice 空壳槽与 Esc 关槽
- [x] 5. 拆 `bridge/handlers`；桥测与 compaction/retry 不回归
- [x] 6. 单测：槽互斥 / Esc 关槽 / busy Esc 不开树；扩既有 `tests.rs`/`harness`
- [x] 7. 更新 `src/app/tui/AGENTS.md` 模块指针
- [x] 8. `llman sdd validate c494-refactor-app-tui-editor-slot --strict --no-interactive` + `just qa`
