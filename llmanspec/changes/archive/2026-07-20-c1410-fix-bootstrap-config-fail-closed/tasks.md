# Tasks: c1410-fix-bootstrap-config-fail-closed

- [x] 1. live specs：强化 `cli-entry` ce2 + 新增 req；`runtime-model-registry` 边界 req；对应 `.feature` 场景
- [x] 2. `BootstrapError` + `resolve_assembly`：ConfigLoadFailed / 零模型硬失败；收紧 env 自动选中
- [x] 3. CLI 错误呈现；TUI/未选中展示 `NOT-SET`
- [x] 4. 单测覆盖失败路径与未选中；修 `.xylitol/config.yaml` 注释模板陷阱
- [x] 5. `llman sdd validate` + 相关 `cargo test` / `just qa` 子集；finalize 后本地 merge
