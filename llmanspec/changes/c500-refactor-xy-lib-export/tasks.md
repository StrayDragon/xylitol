# Tasks — c500-refactor-xy-lib-export

- [ ] 1. 起草精选 `pub use` 符号表（对照 `_NOTE.md` §2.1），写入 `src/lib.rs` 注释 + 实际 re-export
- [ ] 2. 删除 `SessionIO`；`Agent` 直接用 `store`；删未用的 `XySessionStore::{load_context,append_entry}` 若仅被 SessionIO 使用
- [ ] 3. 删除 `PermissionGate`；`Agent` 直接持有 `Arc<dyn XyPermission>`
- [ ] 4. 删除 `LlmMessageConverter`、`XyToolDefinition`
- [ ] 5. 清理 `InProcessDriver.model_builder` 死字段（若仍无读）
- [ ] 6. 更新/确认根与 `src/AGENTS.md` 与 ar06 语义一致
- [ ] 7. `llman sdd validate c500-refactor-xy-lib-export --strict --no-interactive`
- [ ] 8. `just lint` + 相关 `cargo test`（arch_guard、agent session）
