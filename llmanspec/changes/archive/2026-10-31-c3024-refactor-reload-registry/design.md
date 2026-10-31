# 设计：重载参与者注册表

## 运行时

- `ReloadContext`：`cwd` / `agent_dir` / `project_trusted`。
- `ReloadParticipant`：`name() -> &'static str`；`reload(&mut XyInProcessDriver, &ReloadContext) -> Result<String, String>`。
- `ReloadRegistry`：注册与快照；驱动字段默认空注册表。
- 参与者失败 → 步骤 `ok=false`，继续执行后续参与者；取消 → 返回已收集步骤并标记 cancelled。

## 内核

- `ProductReload::reload`：取 cwd/agent_dir/trust，遍历注册表，逐项执行并收集 `ReloadStepReport`。
- 内置参与者：
  - `skills`：`bootstrap::reload_skills`；
  - `mcp`：报告 session/prepare 已触发（prepare 在 `reload_runtime` 前置触发）；
  - `context`：`bootstrap::reload_prompt_context`；
  - `models`：重读配置、重建注册表、替换；空注册表保留旧表并报失败。
- 注册点：`assemble_driver_with_ops_and_features_and_context`（所有宿主驱动共用）。

## 规格

- `app-tui-commands` reload 规则改为：触发已注册的宿主热重载机制与端侧本地机制（keybindings、themes），汇总各步骤诊断。

## 风险

- 步骤顺序必须保持（skills → mcp → context → models），由注册顺序保证；单测锁定。
