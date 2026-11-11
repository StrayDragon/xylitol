---
depends_on:
  - c3240-fix-react-loop-run-state
  - c3250-fix-server-slot-lifecycle
title: "Fix loop middleware lifecycle resets and automation transitions"
---

# Loop 中间件生命周期复位与自动化状态转移修复

> 2026-11-11 定稿（按 c3240 词汇：session ⊃ turn（问答轮）⊃ step；run 为宿主实现术语）。依赖 c3240（词汇与 BeforeStepCtx.has_new_user_messages）与 c3250（automation admit 单一入口）。

## Why

- **generator-critic 终态跨轮泄漏**（loop-policies/middleware.rs:255-365 已核证）：进入 Approved / Exhausted 后无回 Idle 转移、rounds 永不复位；decide_continue 对 Approved 直接 stop——用户第二个问题起（含 follow-up 续问）模型得不到执行机会。
- **todo checkpoint 闸门先置位后判断**（todo/middleware.rs:425-455 已核证）：swap(true) 先于 session_id 判定；首调无 session 则 restored 永久 true，断点恢复永不发生且无自愈。
- **goal / schedule 转移全开放**（tools/automation.rs:170-230 已核证）：set_state 只校验存在性，任意态可到任意态（Cancelled→Running 可达、终态后仍可 Pause）；Exhausted 是投影派生与持久状态两套真值。
- todo reminder 三原子量无一致快照；进度按工具输出 status 字符串重解析（SDK 另有 TodoStatus 枚举，词汇分叉）。

## What Changes

- critic 复位 = **turn 边界**（BeforeStep && has_new_user_messages）：新问答轮即新评审回合——直接修复「第二问被立刻 stop」，覆盖 follow-up 续问；rounds / unparsed / instruction_injected 同步复位，单一 reset 入口。
- todo 闸门改「先判后置」；restore 失败回滚 restored 并可重试。
- goal / schedule 合法转移表（矩阵见 design §2）：工具写入前校验拒绝非法（LLM 可见错误）；存量事实流容忍投影（保留最后合法态 + warn）；Exhausted 事实化（tick 引擎写 ScheduleStateChanged）。
- todo reminder 三原子合并单 Mutex 快照；进度解析改用 SDK TodoStatus 枚举。
- 边界：automation admit 单一入口归 c3250（依赖）；本 change 只做转移表与 tick 内 skip 规则。

## Capabilities

- agent-runtime（loop 中间件复位边界）；agent-todo（闸门 / reminder）；agent-goal + agent-automation（转移合法性）。

## Impact

- 范围：plugins/xylitol-official-plugin-{loop-policies,todo}、tools/automation.rs、runtime agent/{automation,goal}.rs。行为变更：critic 第二问恢复执行（用户可见修复）；非法 set_state 显式拒绝。

## 决策记录（2026-11-11，全部已决）

1. （已决）critic 复位 = turn 边界（has_new_user_messages），非提交边界——「第二问」包含 follow-up 续问。
2. （已决）todo 闸门先判后置 + 失败回滚可重试。
3. （已决）转移表拒绝非法 + 存量容忍投影；Exhausted 事实化。
4. （已决）reminder 单 Mutex 快照；进度解析用 SDK TodoStatus。
5. （已决）automation admit 归 c3250，本 change 不重复实现。
