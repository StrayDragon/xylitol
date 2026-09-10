# Design: 删除产品死旋钮

> Designed / pre-start。依赖 `c2710`（Command 已是 SSOT）。

## 1. DatePlacement

事实：`src/agent/context_policy/mod.rs` 枚举；产品默认 `Omit`。`prompt/system.rs` 与 `capabilities/prompt_ops.rs` 仍实现 pinned/as-today。

落地：类型删除；system prompt 不再插入 Current date/cwd（除非 session_env 已提供，保持现状 Omit）。测试只留 Omit 路径。

配置：`rg date_placement|DatePlacement`；YAML 键若存在则从 schema/example 生成器删除。

## 2. cycle_thinking_level

事实：`XyDriver` 明确 legacy；产品 TUI `/model`。harness 与 `tui/tests.rs` `models_picker_left_right_cycle_thinking_levels` 可能测的是 picker 左右，**不要**误删 picker，只删 Driver 循环方法。

若左右键调用的是 Driver.cycle_thinking，改为与 `/model` 同一 `SetThinkingLevel` 路径。

## 3. 验证

产品 TUI：改 thinking 仍只经模型面板。`cargo test` 删除消融测。`just qa`。
