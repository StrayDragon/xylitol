---
depends_on: []
title: "Fix P0 batch review follow-ups"
---

# P0 批次复核收尾（c3266）

> 2026-11-12 三组只读 review + 本机复核的产出。上游 9 个 change 均已归档合入，本 change 承接全部遗留：1 CRITICAL、8 WARNING、checkpoint 短路、v1 门禁回收与卫生项。W7（HistoryRow 落盘顺序）待用户决策后定稿入任务。

## Why

P0 批次主体成立（qa 3727/3727、validate 全绿、双 rg 归零），但复核发现：c3265 的 workflow_write 工具可绕过转移表直改任务状态（C1，违反已落地 r2966）；另有 8 项 WARNING 与一批测试/卫生缺口。逐项前后与优劣见 design §1-§10。

## What Changes

- C1：workflow_write 逐任务状态写入经 task_transition 校验（replan 允许集例外）。
- W1：RunCoordinator transition 单入口 + mark_streaming 不得覆写 Cancelling（补 c3240 design 承诺、tasks 漏列项）。
- W2：goal/schedule 投影按转移表过滤非法事实（保留最后合法态 + warn）。
- W3：TodoStatus 未知值显式报错（错误回模型自纠）。
- W5：manifest 脏标记 + turn 边界批量落盘（消灭逐 entry 双 fsync）。
- W6：Completions 适配器终态矩阵 SSE 回放测试。
- W8：session_shutdown 接线为「下一次 session_start 补记」；session_start 谓词改 first_of_session（每 run 语义归 run_start）。
- W9：run_command_capture 超时杀子进程（kill_on_drop + 进程组）。
- checkpoint：解析失败短路返回 None + warn（不再回扫更旧条目）。
- 卫生项：v1 门禁回收；golden 夹具生成脚本；drift stat 快路径；context_reminder 行持久化→投影端到端 BDD；T8 示例注入器；critic follow-up 变体与转移矩阵单测；compaction 快照保留测试补全；decide_active_goal 内部转移走表；勘误记录（BeforeStepCtx 实为 4 字段含 session_id、trust 条款落点 app-tui-trust）入 research。
- 待决（W7）：① 改约（提醒行紧随用户消息，零代码）② 修序（评估前移到用户消息落盘前，取消 fragment 双通道，fork 安全）——推荐 ②，待用户拍板后并入任务。

## Capabilities

- agent-workflow（C1）；agent-runtime（W1/W8/W9）；agent-goal + agent-automation（W2）；agent-todo（W3）；agent-session-store（W5/checkpoint）；infra-provider（W6）；agent-prompt（W7 若 ②）；runtime-config（v1 回收涉及门禁脚本，非 spec）。

## Impact

- 范围：均为已合入代码的小修与补测；无 breaking；无新词表。

## 决策记录（2026-11-12）

1. （已决）C1 经表校验是唯一方案（备选「禁 write 收 status」破坏合法 replan 写入，不取）。
2. （已决）W1 补完整 transition 单入口（非仅加守卫）——按 c3240 原设计。
3. （已决）W3 解析失败错误回模型（备选 warn+pending 违反 agent-todo.feature:139，不取）。
4. （已决）W8 补记方案（备选装载期报错把负担丢给用户，不取）；session_start 改 first_of_session。
5. （已决）checkpoint 短路 + warn（备选 serde alias 兼容违反「不留兼容解析」已决口径，不取）。
6. （已决）v1 门禁回收含在本 change。
7. （待决）W7 顺序：推荐 ② 修序（fork 场景提醒丢失 + 消灭双通道）；① 零代码改约。定稿后更新本记录与 tasks。
