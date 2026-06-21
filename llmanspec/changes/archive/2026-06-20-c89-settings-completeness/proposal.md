---
depends_on:
  - c83-http-dispatcher
---

# c89-settings-completeness: 补齐 Settings 缺失字段

## Why
pi 的 `Settings` 接口有 40+ 个顶层字段；xylitol 当前只有 18 个（见 `src/infra/settings/types.rs`）。排除遥感/插件/TUI 专属字段后，仍有 16 个字段缺失。这导致：
- `transport` / `steeringMode` / `followUpMode` — c80 AgentLoop 预期读取这些配置，但 settings 没有定义
- `shellPath` / `shellCommandPrefix` — bash executor (c95) 期望自定义 shell 路径
- `defaultProjectTrust` — TrustManager 使用的默认信任策略
- `enableSkillCommands` — 技能命令开关
- `markdown` / `warnings` — 格式化控制
- `sessionDir` — 自定义会话存储目录
- `httpProxy` / `httpIdleTimeoutMs` — c83 的配置入口
- `prompts` / `themes` — 用户自定义资源路径

## What Changes
- 扩展 `src/infra/settings/types.rs`：
  - `Transport` 枚举（auto / sse / direct）
  - `Mode` 枚举（all / one-at-a-time）用于 steeringMode / followUpMode
  - `DefaultProjectTrust` 枚举（ask / always / never）
  - `MarkdownSettings { codeBlockIndent: Option<String> }`
  - `WarningSettings { anthropicExtraUsage: Option<bool> }`
  - 新增顶层字段（全部 `Option<>`）：
    - `transport`, `steering_mode`, `follow_up_mode`
    - `shell_path`, `shell_command_prefix`, `npm_command: Option<Vec<String>>`
    - `default_project_trust`, `enable_skill_commands`
    - `prompts: Option<Vec<String>>`, `themes: Option<Vec<String>>`
    - `session_dir`
    - `http_proxy`, `http_idle_timeout_ms`, `websocket_connect_timeout_ms`
    - `markdown`, `warnings`
- 更新 `deep_merge` 处理新字段
- 在 `SettingsManager` 上新增 accessor 方法
- `#[serde(rename_all = "camelCase")]` 对齐 pi wire 格式

## Capabilities
- runtime-config

## Impact
- 非破坏性：所有新字段都是 `Option`，默认 `None` 即不启用。不影响现有 settings.json 向后兼容。
- c83 必须先行（`httpProxy` 等字段在本变更正式定义名称和类型）。
