---
depends_on: []
---

# c81-config-value-resolver: 配置值解析引擎（Shell 命令 + 环境变量模板）

## Why
pi 的 `resolve-config-value.ts`（285 LOC）是 auth 系统的基石——API key 和 HTTP header 的值可以是：
- 普通字面量
- `$ENV_VAR` / `${ENV_VAR}` 环境变量插值
- `!command` shell 命令执行（缓存进程生命周期）

xylitol 的 `auth_storage.rs` 和 `registry.rs` 仅支持直接读取 `process.env`，无法处理 `ANTHROPIC_API_KEY=$PROJECT_KEY` 或 `!pass show api-key` 这类 pi 配置格式。这导致用户无法复用现有的 `.xylitol/auth.json` 和 `settings.json`。

## What Changes
- 新增 `src/agent/config_value.rs` 解析引擎：
  - `ConfigValueReference` 枚举（Literal / EnvVar / ShellCmd）
  - `parse_config_value(config) -> ConfigValueReference`
  - `resolve_config_value(config, env) -> Option<String>`
  - `resolve_config_value_or_throw(config, desc, env) -> String`
  - `resolve_headers(headers, env) -> HashMap<String, String>` — 逐 value 解析 header
  - `get_missing_config_value_env_var_names(config) -> Vec<String>` — 暴露缺失变量名
  - `clear_config_value_cache()` — 测试用
- Shell 命令执行使用 `std::process::Command` + 10 秒超时 + 结果缓存
- 集成到 `auth_storage.rs` 的 `get_api_key()` 和 `registry.rs` 的 header 解析
- 同步暴露测试辅助函数

## Capabilities
- provider-integration

## Impact
- 非破坏性：新增模块；auth_storage 和 registry 的方法原地增强，签名不变。
- 无新依赖（仅用 std::process）。
- c82 (provider-attribution) 和 c89 (settings-completeness 中 httpProxy) 将依赖本变更。
