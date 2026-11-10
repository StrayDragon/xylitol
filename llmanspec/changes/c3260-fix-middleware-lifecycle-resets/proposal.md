---
depends_on: []
title: "Fix loop middleware lifecycle resets and automation transitions"
---

# Loop 中间件生命周期复位与自动化状态转移修复

## Why

官方 loop 中间件与自动化状态机的会话生命周期管理存在「一次性闸门置位过早 / 终态不复位 / 转移全开放」三类缺陷（扫描证据见 research/implicit-state-machines.md，C2/C8/C11/C14/C15/A13/F3；Lead 已复核 critic 转移、todo 闸门、goal_set_state 原文）：

1. **generator-critic 终态跨轮泄漏**（loop-policies/middleware.rs:117-151）：进入 Approved/Exhausted 后无回 Idle 转移、rounds 永不复位；decide_continue 对 Approved 直接 stop——**首轮 APPROVE 后，后续轮次被立刻 stop，第二个问题起模型基本得不到执行机会**。
2. **todo checkpoint 闸门先置位后判断**（todo/middleware.rs:427-449）：swap(true) 先于 session_id 判定执行；首次调用若无 session_id，restored 永久 true，断点恢复永不发生且无自愈。
3. **goal/schedule 转移全开放**（tools/automation.rs:174-224，已复核）：goal_set_state / set_state 只校验存在性，任意态可到任意态（Cancelled→Running、Done 之后 Pause）；Exhausted 是投影派生与持久状态两套真值。
4. automation tick 与 drain_pending 两条独立放行路径；todo reminder 三原子量无一致快照、进度按 status 字符串重算且未知值静默归 pending。

## What Changes

- critic 状态随 run 边界复位（或 turn 边界，见 Open Questions）；Approved/Exhausted 消费一次后回 Idle；rounds 复位；instruction_injected 单点 reset。
- todo checkpoint 闸门改为「先判条件再置位」，或失败路径回滚 swap；restore 失败可重试。
- goal/schedule 引入合法转移表（事件级校验：终态不可逆、Paused↔Active 显式、Exhausted 派生与持久合一）；LLM 工具 set_state 拒绝非法转移并说明。
- automation 放行收敛单一入口；todo reminder 与进度解析改单一快照 + SDK TodoStatus 枚举（消灭字符串重解析）。

## Capabilities

- agent-runtime（loop 中间件调度）；agent-todo（checkpoint / reminder）；agent-automation + agent-goal（转移表）。

## Impact

- 范围：plugins/xylitol-official-plugin-{loop-policies,todo}、plugins/xylitol-official-plugin-tools/src/automation.rs、crates/xylitol-runtime/src/agent/{automation,goal}.rs。
- 行为变更：critic 第二问恢复正常执行（用户可见修复）；LLM 对 goal 的非法 set_state 从静默追加变为显式拒绝。
- 验证：critic 两轮对话 BDD；todo 无 session 首调用的恢复能力单测；goal 非法转移矩阵单测。

## Open Questions

1. critic 复位边界：run 结束（用户下一问）还是 decide_continue stop 后立即？
2. goal 转移表严格度：拒绝非法（推荐）还是 warn+clamp（兼容旧会话事实流）？存量会话中已存在的非法序列如何投影？
3. automation 单一入口是否顺带统一 busy 判定来源（run_inflight vs queue 非空）？
