---
depends_on: []
---

# c95-fix-test-stability

## Why

测试稳定性审查发现多处可能导致 CI flaky 的问题：

1. **LSP 测试写入 `/tmp` 永不清理**：固定路径 + 无 RAII，并行测试污染 + 磁盘泄漏
2. **配置加载测试使用固定目录名**：`xylitol_test_no_files` 等在多进程/重跑时可能冲突
3. **~80 个 async 测试无外层超时**：mock/stream 死锁时 CI 挂死到全局超时（60s）
4. **时间敏感测试依赖真实 `sleep`**：高负载 CI 下边界值可能失败
5. **依赖外部 `sh`/`echo`/`sleep` 命令**：minimal 容器或 Windows 下直接失败
6. **`serial_test` 已声明但从未使用**：env 变更测试在并行时存在竞态风险
7. **Insta 快照依赖 UI 渲染细节**：ratatui/vt100 升级即 break
8. **MockToolContext 过于精简**：工具开始读取 workspace_root 时会产生假阳性

## What Changes

1. LSP 测试改用 `tempfile::NamedTempFile` + RAII 自动清理
2. 配置加载测试改用 `tempfile::TempDir` 唯一路径
3. 为关键 async 测试添加统一 `with_test_timeout()` wrapper（默认 10s）
4. bash/hooks 超时测试改用更短 sleep 或 `tokio::time::pause()` + `advance()`
5. 外部命令测试标记 `#[cfg(unix)]`；CI 文档明确依赖
6. 删除或启用 wiremock / serial_test dev-deps
7. Insta 快照添加 filters 忽略 ANSI/空白差异
8. 扩展 MockToolContext 支持可配置 `workspace_root`

## Capabilities

- `test-infra`: 测试基础设施

## Impact

- 测试执行更可靠，CI 超时风险降低
- LSP 测试不再在 `/tmp` 留痕
- 新增 timeout wrapper 需被后续测试采用（可 lint 检查）
