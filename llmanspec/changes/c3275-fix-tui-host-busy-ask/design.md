# Design — TUI busy 状态与 ask 槽

## 1. 目标与边界

- busy 单一真值；ask 槽不可静默覆盖；流式缓冲单点复位。
- 边界：不改 Esc 语义与效果循环结构（PendingOps lane 机制保留）。

## 2. 决策摘要

| 决策 | 内容 | 状态 |
|---|---|---|
| busy 相位 | host 层位集（AgentRun / Bash / Reload 可并存），EnumSet 形态单一字段承载 | 推荐，待 review |
| 收口 | end_bash / end_reload / note_user_abort 全走 recalc_phase() 按剩余活动决定 Idle | 定稿方向 |
| ask 槽 | 占用时后到者拒绝：本地 ask 报 already waiting（已有）；远端 approval 排队容量 1，溢出通知条提示 | 推荐，待 review |
| 流式缓冲 | StreamingBuffers struct，set/flush/reset 单点；session_tree 重建改走 reset | 定稿方向 |
| 结算判据 | reason 改 SettlementReason 枚举（runtime 侧同步导出） | 定稿方向 |

## 3. busy 位集

busy 为 EnumSet<Activity>（AgentRun | Bash | Reload）。begin 置位、end 清位后统一 recalc：位集空则 Idle，否则 Busy + 最高优先级活动文案。reload 中 end_bash 不再误置 Idle（位集仍含 Reload）。UiModel.phase 降为由 host recalc 推送的投影。

## 4. ask 槽占用策略

push_from_server 遇槽占用时入 pending_remote 队列（容量 1）；本地 take_pending 消费后弹出队列；队满覆盖时通知条提示并记日志。

## 5. 测试边界（seam）

- harness：reload 中 bash 结束状态栏仍 Reloading；远端 approval 与本地 ask 并发；流式缓冲重置矩阵。
- BDD：app-tui-host / app-tui-ask 场景回归。

## 6. 风险与回滚

- 回滚：busy / ask / buffers 三片独立。
