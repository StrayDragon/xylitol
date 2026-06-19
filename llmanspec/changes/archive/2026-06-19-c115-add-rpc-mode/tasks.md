# Tasks

- [x] 1. `src/interface/rpc.rs`：RpcCommand 全集 + RpcEvent 类型 + RpcState 管理 + 派发 + 流式 prompt
- [x] 2. 数据结构：RpcCommand（21 个变体）/ RpcEvent 含 id 关联
- [x] 3. 主循环：stdin 逐行读 → dispatch → stdout 逐行写 RpcEvent
- [x] 4. CLI `--rpc` 在 `interface/cli/mod.rs` 路由到 `run_rpc_mode`
- [x] 5. SIGINT/SIGTERM 通过 CancellationToken 传播 → abort + 干净退出
- [x] 6. 集成测试：用管道驱动 stdin/stdout 往返（见下）
- [x] 7. 单元测试：命令解析 / 未知命令错误 / id 关联
- [x] 8. Run `cargo test --lib` 与 BDD
- [x] 9. Run `cargo fmt` 与 `cargo clippy`
- [x] 10. Run `llman sdd validate c115-add-rpc-mode --strict --no-interactive`
- [x] 11. Run `just qa`
