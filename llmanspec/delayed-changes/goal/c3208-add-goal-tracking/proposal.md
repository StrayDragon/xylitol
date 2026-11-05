---
depends_on:
  - c3206-refactor-session-graph
title: "Add goal tracking"
---

## Why

/goal 是任务定义与终止标准：围绕一个目标规划步骤、持续迭代，直到完成或失败。它不是 /loop；/loop 管怎么重复，/goal 管要达成什么。

必须有硬上限，不能无限迭代：goal 事实带 max_iterations（缺省 256，可配置），每轮结束检查是否达成或触达上限。

## What Changes

- Goal 事实：id、branch、objective、acceptance、status（open / running / done / failed / blocked）、max_iterations、iterations_used、evidence。
- 每轮结束（after_turn / decide_continue）给 agent 一次判定机会：标记 done 须附 evidence；未达成则继续，达到 max_iterations 标记 failed / blocked，不得无限续跑。
- 用户可 pause / cancel / 修改 acceptance；状态变更只追加事实，不覆盖。
- 与 /loop 组合：loop 触发检查，goal 判定完成。
- UI：目标、进度、剩余迭代、证据（TUI / nsh 同源）。

## Scope

- 不在 c3206 内实现；c3206 只提供 branch 与事件基础。
- 不做自动改写 acceptance 或无限自我迭代。

## Related

- #28（roadmap 索引；当前无 /goal FR）
- #71（决策复用；可作为 goal 推进的省 token 手段）
- #19（loop 管理，已关闭）

## Open Questions

- acceptance 的结构化格式与证据类型。
- 谁有权标记 done：agent 自评、用户确认，还是两者。
- 与 todo / workflow 的边界：goal 是终止标准，不是步骤清单。
