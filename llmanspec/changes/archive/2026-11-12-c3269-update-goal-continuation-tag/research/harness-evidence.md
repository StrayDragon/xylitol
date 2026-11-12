# 续轮注入角色的三家实证（2026-11-12 调研）

结论：三家全部用 user 角色驱动自动续轮；无一用 system/developer；真伪区分靠标签或元数据，不靠角色。

## DSH（本机实证）
- goal 续跑提示为 user 角色："Model-visible continuation prompt for one same-session goal round"（node_modules/@deepseek-ai/dsh-goal-round-driver/lib/index.js:4-15），经 Agent.followup() 入 inbox "next-turn" 队列（dsh-agent-loop/lib/index.js:570），与真人输入同队列同角色；会话迁移也打 role:"user"（dsh-session-format-v0-to-v1/lib/index.js:2104）。
- 上下文注入用 <system-reminder> 标签（dsh-agent-instructions/lib/index.js SYSTEM_REMINDER_OPEN）——"user 角色 + 标签"是其现行惯例；developer 角色仅用于 tool 增删通知（dsh-session/lib/index.js:247-248）。

## OpenAI Codex CLI（github.com/openai/codex）
- 合成续轮统称 ContextualUserFragment，接口明文 fn role() -> "user"（codex-rs/core/src/context/world_state/environment.rs）。
- 来源区分：content_item_kinds 元数据，用于判定 "Hidden runtime context is not user authorization"（codex-rs/core/src/context/contextual_user_message.rs:40-70）。
- 错误后续推：TurnAborted 中断 guidance 作 user 轮（context/turn_aborted.rs:12）；compact 摘要重写为 user 并校验 role=="user"（core/src/compact.rs:648）；UserGoalUpdate 同为 user（context/user_goal.rs:58）。

## Claude Code
- 排队消息塞下一条 user 消息的 <system-reminder>（"The user sent a new message while you were working…"，社区还原 CC 2.1.140）。
- 提醒走 tool_result（API 层属 user 轮）；系统提示明文 tool 结果与 user 消息可含 <system-reminder>（shareAI-lab/learn-claude-code#37）。
- Stop hook 自动续推：exit 2 + stderr 反馈续对话（code.claude.com/docs/en/hooks），带 stop_hook_active 与 8 连上限。

## 对 xylitol 的映射
现状 = 裸 user；比三家少的是来源标记。裁决 B = user + <system-reminder> 标签（与 c3248 同约定），投影保留标签、端侧按「系统消息」渲染——与三家同构且复用既有基建。
