# Tasks — c1100-update-runtime-context-hot-reload

## 1. Protocol + loader

- [x] 1.1 在 `runtime_protocol` 增加 `XyReloadable`（关联 `Outcome`）并 re-export
- [x] 1.2 `DefaultResourceLoader::reload`：清缓存 + `load_all`；实现 `XyReloadable`
- [x] 1.3 单测：改盘上 AGENTS.md 后 `reload`，`get_agents_files` 反映新内容

## 2. Agent + Driver seam

- [x] 2.1 `AgentCapabilities::apply_prompt_resources` + `AgentRuntime` 转发
- [x] 2.2 `InProcessDriver::apply_prompt_resources`
- [x] 2.3 单测：apply 后 `rebuild` 的 system 含新 context；历史消息条数不变

## 3. app/core 助手

- [x] 3.1 `reload_prompt_context(driver, cwd, agent_dir, trusted, config_system_prompt)`（或等价）
- [x] 3.2 单测：untrusted 时项目 AGENTS 不进入；trusted 时进入

## 4. 校验

- [x] 4.1 `llman sdd validate c1100-update-runtime-context-hot-reload --strict --no-interactive`
- [x] 4.2 `just qa`（或至少 `just lint` + 相关 `cargo test`）
