# Tasks

- [x] 1. `src/agent/templates.rs`：`PromptTemplate` 增 `source_path: Option<PathBuf>`，补 `new()`/构造点为 None
- [x] 2. `src/agent/session.rs`：新增 `register_prompt_commands(&mut self, templates: &[crate::infra::resource::PromptTemplate])`，转换 loader→runtime 类型并注册 + 填 source_path
- [x] 3. `get_commands()`：让已注册 prompt 模板纳入命令列表
- [x] 4. `src/interface/cli/mod.rs::run()`：构建 ResourceLoader 后调用 `register_prompt_commands` 注入
- [x] 5. 单元测试：注入后可被 `process_prompt` 展开命令命中 / get_commands 含模板 / source_path 保留 / 位置参数不变
- [x] 6. Run `cargo test --lib` 与 BDD
- [x] 7. Run `cargo fmt` 与 `cargo clippy`
- [x] 8. Run `llman sdd validate c100-add-prompt-templates --strict --no-interactive`
- [x] 9. Run `just qa`
