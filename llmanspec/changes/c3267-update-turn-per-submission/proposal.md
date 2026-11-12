---
depends_on:
  - c3266-fix-p0-review-followups
title: "Update to one-submission-one-turn admission"
---

# 提交即 turn：顺序单条投递模型（c3267）

> 2026-11-12 对齐定稿。用户裁决：follow-up / steer 不做 batch；「一次提交 = 一个 turn」；steer 保留轮内改向（Option A）；run 概念退役。依赖 c3266（协调器 / 注入点重叠面串行）。

## Why

现状 batch 语义（已核证）：settle 后 drain_queue(follow_up_queue) **一次取全部**排队消息喂给同一个 turn 的同一次模型调用；steer 在 settle 注入同一次提交内的下一轮。后果：① 「一次提交含多个 turn」迫使词汇表供养一个概念层之外的 run（宿主实现术语补丁 + 反向提示，仍反复引起混淆）；② 多条 follow-up 被合并进一个 prompt，回合边界模糊；③ 与业界常态（Claude Code / Codex：排队消息一次一条依次投递、各成独立回合）相悖。

用户模型（定稿）：**session ⊃ turn（= 一次提交 = 一对 AgentStart/AgentEnd）⊃ step**——真三层，无隐藏层；队列 = 待提交 FIFO，settle 后弹一条 → 新 turn。

## What Changes

- **提交即 turn**：每次提交（含 follow-up 弹出）产生独立 turn 与独立 AgentStart/AgentEnd；follow-up 从 drain 全部改为 FIFO 弹一条。
- **steer 保留轮内改向（A）**：turn 在途时，steer 在 step 边界注入**当前 turn**（现 decide_continue 机制）；settle 后到达的 steer 无在途工作可改，视为一次提交（新 turn）。
- **hook 事件第三次更名（语义终点）**：run_start / run_end / run_settled → turn_start / turn_end / turn_settled（历程 agent_* → run_* → turn_*；落在与 XyEvent::AgentStart/End 同一边界的正确终名）。XyEvent 变体名不动（避免与刚改名的 StepStart 重建混淆线）。
- **c3248 注入点跟名**：run_start / run_end 点 → turn_start / turn_end；turn_pre 谓词简化——新模型下每个 turn 都以用户输入开始，has_new_user_messages 每 turn 恰为真一次，判定精确化。
- **实现标识符不动**：RunCoordinator / RunId / run_inflight 等保留（纯实现命名，语义 = 单个 turn 的执行机器）；AGENTS.md 词汇段更新（run 从「宿主实现术语（一个事件流）」改写为「已退役边界的历史实现命名」）。
- **概念层 run 残留清扫并入**：agent-runtime 等五处 spec 条款重写为 turn / 「每次提交」措辞（abort-queue-semantics、准入、事件粒度条款本来就要改，顺带清零）。
- **TUI 连续性**：AgentEnd 粒度变细（每轮一对）——队列非空时 busy 不闪断（QueueUpdate 深度已知）。
- **automation / goal 跟进**：调度 tick 投递 = 一次提交；goal「同 run 续跑」改述「逐 turn 续跑」；机制不变。

## Capabilities

- agent-runtime（准入 / abort-queue-semantics / 事件粒度重写）；agent-hooks（事件名）；agent-prompt（注入点跟名）；agent-goal / agent-automation（措辞）；app-tui-bridge / app-tui-host（busy 连续性与 AgentEnd 粒度）。

## Impact

- breaking ×2（未推广期接受，与整批同一决心）：AgentEnd/AgentStart 粒度变细（每轮一对）；hook 事件名 run_* → turn_*。
- 行为变更：多条 follow-up 从合并单 prompt 变为依次独立回合。

## 决策记录（2026-11-12，全部已决）

1. （已决）提交即 turn；follow-up FIFO 逐条投递，每条独立 AgentStart/AgentEnd。
2. （已决）steer = A 轮内改向：在途注入当前 turn（step 边界）；settle 后到达视为提交。
3. （已决）hook 事件 run_* → turn_*（第三次更名，语义终点）；XyEvent::AgentStart/End 变体名不动。
4. （已决）实现标识符（RunCoordinator 等）本轮不改名。
5. （已决）TUI busy 在队列非空时保持连续，不因每轮 AgentEnd 闪断。
6. （已决）概念层 run 残留清扫并入本 change（非独立卫生项）。
7. （已决）depends_on c3266（协调器与注入点重叠面串行）。
