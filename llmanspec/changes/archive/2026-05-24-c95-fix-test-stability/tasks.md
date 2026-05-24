# c95-fix-test-stability Tasks

- [x] LSP `temp_file()` helper 改用 `tempfile::NamedTempFile`，返回 `(uri, path, guard)` RAII
- [ ] 配置 loader/secret 测试改用 `tempfile::TempDir`（defer - 已有 ENV_LOCK 串行化）
- [ ] 创建 `with_test_timeout` helper（defer - 需要统一 test harness 改造）
- [ ] 为 async 测试包裹 timeout wrapper（defer - 联动上项）
- [ ] bash/hooks 超时测试缩短 sleep（defer - hook 测试已用 50ms）
- [ ] 外部命令测试标记 `#[cfg(unix)]`（defer - 低优先级）
- [ ] 审查并删除未使用的 dev-deps（defer）
- [ ] MockToolContext 添加 `workspace_root`（defer - 与工具重构联动）
- [ ] 补充 paths fallback 测试断言（defer）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c95-fix-test-stability --strict --no-interactive`
