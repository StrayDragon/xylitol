# Tasks: c1440-refactor-migrate-builtin-tools-typed

- [x] 1. 迁移 `grep` → `TypedTool`（`GrepArgs` pub；行为字面保持）
- [x] 2. 迁移 `write` → `TypedTool`（含 `prompt_guidelines`）
- [x] 3. 迁移 `edit` → `TypedTool`（含 Sequential `execution_mode` + guidelines）
- [x] 4. 迁移 `bash` → `TypedTool`（含 guidelines）
- [x] 5. 迁移 `read` → `TypedTool`（`execute_as_parts_typed` + `execute_typed` 委托）
- [x] 6. 单测经 `XyTool::execute` / `execute_as_parts` 仍绿；`cargo test -p xylitol --lib infra::tools`（或等价）通过
- [x] 7. 可选：更新 `src/AGENTS.md` 工具段（若仍写「样板进度」则对齐）；proposal 去掉 deferred
- [x] 8. `just qa`（或至少相关测 + lint）绿
