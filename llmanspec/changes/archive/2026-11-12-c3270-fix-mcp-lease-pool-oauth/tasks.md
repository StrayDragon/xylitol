# Tasks — MCP 租约池与 OAuth 终态修复

> 实施者导读：design §7 是复核结论（含两处更正：无 gateway 不产生悬空 Pending；prepare_acked 缺口在 new/fork 而非 switch_session）。收口走 finalize，不列为任务。

- [x] T1 spec delta：infra-mcp r3294 补租约相位 / 关闭任务取消 / idle_closes；r3270 补 OAuth 终态必达；agent-runtime r1052 补 new/fork 门禁复位
- [x] T2 租约相位：PoolEntry 增 phase（Leased / IdleGrace / Closed）；acquire/release/expire 显式转移；schedule_idle_close 返回 JoinHandle 存 entry，新 acquire / reset_pool abort；idle 过期计入独立 idle_closes；单测（相位转移、任务取消、idle_closes 不计 reconnect）
- [x] T3 快照单点：last_view / last_result / last_tools 只由 update_snapshot 写；view() try_lock 失败返回 degraded（complete=false）而非假就绪；SessionPrepareView 增 degraded；单测
- [x] T4 OAuth 终态：status 单真值（存 status + expires_at_ms）；监听器失败 / 提前回调失败 / gateway 缺失 / 查询过期 均落 Failed 并发事件；cancel 落 Cancelled；失败矩阵单测
- [x] T5 进度代际：McpConnectProgress 增 generation；hub 每批 connect 取新代际；旧代际写入丢弃；单测
- [x] T6 门禁复位：new_session / fork_session 清冻 + fire_session_prepare（重置 prepare_acked）；单测（新会话首轮再次门禁）
- [x] T7 回归：just fmt / lint / test / qa；llman-sdd validate c3270 --strict 绿
