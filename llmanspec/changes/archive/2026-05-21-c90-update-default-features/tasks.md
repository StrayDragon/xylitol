## Tasks

- [x] 修改 `Cargo.toml` `[features]` 中 `default` 列表，加入所有非 dev feature
- [x] 运行 `cargo build` 确认全功能编译通过
- [x] 运行 `cargo build --no-default-features --features ui-tui` 确认最小化构建通过
- [x] 运行 `just test` 确认测试通过
- [x] 运行 `llman sdd validate c90-update-default-features --strict --no-interactive` 确认 spec 校验通过
