# 设计：端侧重载参与者

## 类型

- `ClientReloadPhase`：`BeforeHost` / `AfterHost`。
- `ClientReloadStep<T: Terminal>`：`name`、`phase`、`run: Box<dyn Fn(&mut HostSession<T>) -> Result<String, String>>`。

## 运行器

- `run_interactive_reload(..., steps: &[ClientReloadStep<T>])`：
  1. 执行 `BeforeHost` 步骤；
  2. 宿主重载（`driver.reload_runtime`）；
  3. 执行 `AfterHost` 步骤；
  4. 刷新目录与滚动提示。
- 步骤失败 → 该行记为 `{name}: failed — {e}`，继续执行。

## 默认步骤

- `default_client_reload_steps()`：keybindings（`BeforeHost`）、themes（`AfterHost`）。
- host 循环与 harness 在调用前构建并传入。

## 文档

- 不写 `src/AGENTS.md`：注册表类型与默认步骤在代码中可见。
