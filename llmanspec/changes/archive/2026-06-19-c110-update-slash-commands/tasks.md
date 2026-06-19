# Tasks

- [x] 1. 重写 `src/agent/commands.rs`：`SlashCommandInfo`（含 source/source_info）+ `SlashCommandSource` + 22 builtin 表
- [x] 2. `src/agent/session.rs`：`dispatch_slash_command()` + `process_prompt` 优先派发
- [x] 3. 接入 `/export`→c105、`!`/`!!`→c95、`/prompt`→c100、`/compact`/`/fork`/`/tree`/`/new`/`/resume` 等已有 handler
- [x] 4. TUI 专属命令（settings/scoped-models/changelog/hotkeys/trust UI）返回 `NotAvailable` 引导
- [x] 5. 一次性更新所有旧 `SlashCommandInfo`/`BUILTIN_COMMANDS` 调用方与测试（不做 BC shim）
- [x] 6. 单元测试：22 builtin 存在性 / source 字段 / dispatch 路由 / TUI 不可用分支
- [x] 7. Run `cargo test --lib`
- [x] 8. Run `cargo fmt` 与 `cargo clippy`
- [x] 9. Run `llman sdd validate c110-update-slash-commands --strict --no-interactive`
- [x] 10. Run `just qa`
