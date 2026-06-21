# Tasks

- [x] 1. 新建 `src/agent/http_dispatcher.rs`
- [x] 2. 实现 `parse_http_idle_timeout_ms(value) -> Option<u64>` 解析函数
- [x] 3. 定义 `HTTP_IDLE_TIMEOUT_CHOICES` / `DEFAULT_HTTP_IDLE_TIMEOUT_MS` 常量
- [x] 4. 实现 `apply_http_proxy_settings(http_proxy)` — 设置 HTTP_PROXY/HTTPS_PROXY env
- [x] 5. 实现 `configure_http_client(builder, timeout_ms, proxy)` — 在 reqwest::ClientBuilder 应用设置
- [x] 6. 在 `Settings` types 中新增 `httpProxy`/`httpIdleTimeoutMs`/`websocketConnectTimeoutMs` 字段（Option）
- [x] 7. 集成到 `agent/provider/mod.rs`：构建 Client 时调用 `configure_http_client`
- [x] 8. 编写单元测试覆盖超时解析、代理 env 设置
- [x] 9. `cargo test --lib` 全绿（408 passed）
- [x] 10. `cargo clippy` 无新增警告
