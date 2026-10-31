# 设计：模型热重载

## 运行时

- `ModelManager::replace_registry(registry)`：
  - 记录当前模型 id 与 thinking level；
  - 替换 registry；
  - 目标模型 = 当前 id（存在时）→ `preferred_default`（存在时）→ 首个模型；
  - 用 `select_model` 选中目标；thinking level 在新模型支持时恢复，否则用 `default_thinking_for_current`；
  - 无模型时清空 `current_index`，thinking 置 `off`。
- `AgentCapabilities::replace_model_registry` 经 `with_models_mut` 调用。
- `AgentRuntime::replace_model_registry` 转发到 inner。

## 内核

- `bootstrap::build_model_registry` 提升为 `pub(crate)`。
- `ProductReload::reload` 在 `context` 步骤后新增 `models` 步骤：
  - 配置加载失败或注册表为空：`ok: false`，保留旧注册表，message 说明；
  - 成功：`driver.agent.replace_model_registry(registry)`，message 为模型数。

## TUI

- 重载成功后刷新 `model_arg_catalog`（`driver.available_models()`）；当前模型由 `sync_fixed_zone` 自然更新。
- 重载步骤报告沿用既有 `Reload:` 汇总。

## 失败语义

- 配置读取失败：`models` 步骤 `ok: false`，不改变注册表。
- 新注册表为空：`ok: false`，不改变注册表。
- 当前模型失效：回落并在步骤 message 中说明。
