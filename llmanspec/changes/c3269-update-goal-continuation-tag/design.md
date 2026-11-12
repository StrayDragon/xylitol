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

- app-tui bridge：检测 user 行首尾 system-reminder 标签 → 按「系统消息」固定词样式渲染（滚动区样式，非用户气泡）；剥标签后展示正文。
- print：同检测，前缀样式输出；标签不进输出正文。
- 与 c3248 reminder 渲染共用同一检测函数（词表统一，一处实现）。

## 3. spec delta（landing 时）

- agent-goal r3301：补「须携带来源标记（system-reminder 约定）；端侧须按系统消息呈现，不得伪装为真人输入样式」。
- app-tui-transcript / app-tui-fixed-zone：系统消息渲染条款补该来源；cli-print 一句。

## 4. 测试边界

单测：消息构造含标签；投影保留。BDD：goal 续跑场景断言 transcript 渲染为系统消息样式（harness 快照）；print 输出断言。

## 5. 风险与回滚

- 风险：模型对标签内容的权重变化（三家已验证该形态有效，风险低）；标签串撞用户正文——用行首尾整包匹配而非子串。
- 回滚：消息构造 / 渲染 / spec 三片独立。
