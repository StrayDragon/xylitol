# Tasks

- [x] 1. `src/infra/session/export/mod.rs::render_html`：消息/工具结果/bash → 可读 HTML（转义 + entry 块；三函数合为单文件以保持模块紧凑）
- [x] 2. `render_jsonl`：逐行 JSON（同文件）
- [x] 3. `parse_jsonl`：header 校验 + entry 反序列化 + 空输入拒收（同文件）
- [x] 4. `src/agent/session.rs`：`export_to_html` / `export_to_jsonl` / `import_from_jsonl` / `share_as_gist`（stub + 引导）
- [x] 5. `src/infra/session/mod.rs` 导出 export 模块
- [x] 6. 单元测试：html / jsonl 往返 / import 版本校验 / share 未配置引导
- [x] 7. Run `cargo test --lib`
- [x] 8. Run `cargo fmt` 与 `cargo clippy`
- [x] 9. Run `llman sdd validate c105-add-export-capabilities --strict --no-interactive`
- [x] 10. Run `just qa`
