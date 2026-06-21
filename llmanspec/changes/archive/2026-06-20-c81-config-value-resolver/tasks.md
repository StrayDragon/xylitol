# Tasks

- [x] 1. 新建 `src/agent/config_value.rs`，定义 `ConfigValueReference` 枚举与解析函数
- [x] 2. 实现 `parse_config_value(config)` — 识别 `!cmd`、`$VAR`、`${VAR}`、`${VAR:-default}`、普通字面量
- [x] 3. 实现 `resolve_config_value(config, env) -> Option<String>` 与 shell 命令执行（缓存 + 10s 超时）
- [x] 4. 实现 `resolve_headers(headers, env) -> HashMap` 与 `resolve_config_value_or_throw()`
- [x] 5. 实现 `get_missing_env_var_names()` / `clear_cache()` 辅助函数
- [x] 6. 集成到 `auth_storage.rs::get_api_key()`：通过 `resolve_config_value` 解析配置值
- [x] 7. 集成到 `registry.rs::ProviderConfig`：构建模型请求时调用 `resolve_headers` 注入自定义 header
- [x] 8. 编写单元测试覆盖：字面量、env var、shell cmd、缓存、超时、缺失变量检测
- [x] 9. `cargo test --lib` 全绿（381 passed）
- [x] 10. `cargo clippy` 无新增警告
