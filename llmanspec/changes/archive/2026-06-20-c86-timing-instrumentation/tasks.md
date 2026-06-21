# Tasks

- [x] 1. 新建 `src/infra/timing.rs`，定义 `TimingCollector` 结构体（defer）
- [x] 2. 实现 `reset_timings()` / `time(label)` / `print_timings()` — 条件启用（defer）
- [x] 3. 在 `interface/cli/mod.rs` 的 `run()` 开头调用 `reset_timings()`（defer）
- [x] 4. 在 config 加载后插入 `time("config.load")`（defer）
- [x] 5. 在 ResourceLoader.reload() 后插入 `time("resource_loader.reload")`（defer）
- [x] 6. 在 ModelRegistry 加载后插入 `time("model_registry.load")`（defer）
- [x] 7. 在 SessionManager 恢复后插入 `time("session.restore")`（defer）
- [x] 8. 在 AgentSession 创建后插入 `time("session.create")`（defer）
- [x] 9. 在 `run()` 末尾调用 `print_timings()`（defer）
- [x] 10. `cargo test --lib` 全绿（414 passed）（defer）
- [x] 11. `cargo clippy` 无新增警告（defer）
