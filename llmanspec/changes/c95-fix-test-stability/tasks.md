# c95-fix-test-stability Tasks

- [ ] LSP `temp_file()` helper 改用 `tempfile::NamedTempFile`，返回 `(uri, _guard)` RAII
- [ ] 配置 loader/secret 测试改用 `tempfile::TempDir`
- [ ] 创建 `src/infra/config/test_support.rs` 中的 `with_test_timeout(duration, future)` helper
- [ ] 为 agent loop / hooks / harness 测试包裹 timeout wrapper
- [ ] bash/hooks 超时测试缩短 sleep 时长（如 100ms + 50ms timeout）
- [ ] 外部命令测试标记 `#[cfg(unix)]`
- [ ] 审查并删除未使用的 dev-deps（wiremock / serial_test / assert_cmd）或补充对应测试
- [ ] MockToolContext 添加 `workspace_root` 字段（builder 模式）
- [ ] 补充 paths fallback 测试的实际断言
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c95-fix-test-stability --strict --no-interactive`
