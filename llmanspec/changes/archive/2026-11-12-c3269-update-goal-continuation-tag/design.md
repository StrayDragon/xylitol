# Design — goal 续跑来源标记（全部已决 2026-11-12）

## 0. 业界依据（详见 research/harness-evidence.md）

三家 harness 续轮注入对照：DSH = user + followup 队列 + system-reminder 标签惯例；Codex = user（接口明文）+ ContextualUserFragment 元数据；Claude Code = user 轮内 system-reminder（排队消息 / Stop hook 续推）。结论：角色 user 是唯一定论；来源标记是三家共有的完整性要素。

## 1. 消息形态

前（现状，react/mod.rs goal enqueue）：
~~~text
user_parts("目标尚未达成，继续推进。\n{guidance}")
~~~
后：
~~~text
user_parts("<system-reminder>目标尚未达成，继续推进。\n{guidance}</system-reminder>")
~~~

- 标签与 c3248 context_reminder 同款开闭约定；投影给模型保留全文（标签即上下文信号）。
- 单测：goal_continue_delivers_guidance_as_new_turn 断言更新为含标签；新增「真人输入不含标签」不变量断言。

## 2. 端侧渲染

- app-tui bridge：整包匹配（trim 后首尾前缀/后缀同时命中），两处共用同一检测：
  1. user 角色消息（历史重建与 live `MessageStart`）→ `UiEntry::SystemMessage`（剥标签）；
  2. `custom_type = context_reminder` 且 display 为真的 `CustomMessage` 行 → 同一 `UiEntry::SystemMessage`。
  命中即按系统消息呈现：scrollback 以 muted `system: ` 前缀绘制（滚动区样式，非用户气泡）；未命中仍为 `UiEntry::User`，真人输入样式不变。
- 检测实现：`bridge/model.rs` 的 `system_reminder_body` + `user_entry`，两处共用（一处实现）。
- print：核实 print 只渲染流式事件（TextDelta / 工具 / 错误 / 归因），**不回显 user 行**，故续跑指导不出现在 print 输出；不加 print 渲染代码，spec 补 `no-user-echo` 条款。

## 3. spec delta（landing 时）

- agent-goal r3301：补「须携带来源标记（system-reminder 约定）；端侧须按系统消息呈现，不得伪装为真人输入样式」。
- app-tui-transcript / app-tui-fixed-zone：系统消息渲染条款补该来源；cli-print 一句。

## 4. 测试边界

单测（harness 层）：消息构造含标签（runtime `goal_continue_delivers_guidance_as_new_turn`）；桥接两路径均产 SystemMessage（历史重建 + live MessageStart）；真人输入（含子串标签）仍为 User；整包匹配边界。未加 BDD 场景：渲染检测属组件边界，按仓库测试分层由 harness/单测覆盖；新增 spec 规则为裸规则（review pending 计量）。

## 5. 风险与回滚

- 风险：模型对标签内容的权重变化（三家已验证该形态有效，风险低）；标签串撞用户正文——用行首尾整包匹配而非子串。
- 回滚：消息构造 / 渲染 / spec 三片独立。
