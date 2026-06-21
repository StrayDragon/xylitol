# Tasks

- [x] 1. 新建 `src/infra/source_info.rs` 定义通用 `SourceInfo` / `SourceScope` / `SourceOrigin`（defer）
- [x] 2. 实现 `create_source_info()` / `create_synthetic_source_info()` 工厂函数（defer）
- [x] 3. 导出到 `src/infra/mod.rs`（defer）
- [x] 4. 替换 `infra/skills/loader.rs` 中的旧 `SourceInfo` 为通用类型（defer）
- [x] 5. 替换 `infra/resource/loader.rs` 中 `PromptTemplate.source_path` 为 `SourceInfo`（defer）
- [x] 6. 替换 `agent/commands.rs` 中 `SlashCommandInfo.source_path` 为 `SourceInfo`（defer）
- [x] 7. 更新所有引用旧 SourceInfo 的测试（defer）
- [x] 8. `cargo test --lib` 全绿（414 passed）（defer）
- [x] 9. `cargo clippy` 无新增警告（defer）
