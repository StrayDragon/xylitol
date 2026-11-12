# Tasks — TUI 端 busy 状态与 ask 单槽修复

> 实施者导读：design §7 是复核结论（流式缓冲「六文件手工清」部分不成立，已有单一复位入口；PendingOps lane 裁定不在本单）。收口走 finalize，不列为任务。

- [x] T1 spec delta：app-tui-host r3326（busy 活动位集）与 r3329（结算枚举）；app-tui-ask r3327（远端排队/溢出）；app-tui-bridge r3328（streaming 单一复位入口）
- [x] T2 host busy：HostBusy 位集 + recalc_busy 唯一收口；end_bash / end_reload / note_user_abort / run 收尾改走收口；harness 单测（reload 中 bash 结束、reload 中 abort、reload 结束有 run）
- [x] T3 ask 槽：pending_remote 容量 1 + 溢出拒绝响应 + 通知条 + 日志；take_pending 提升；host 通知 drain；单测
- [x] T4 流式缓冲与结算：lifecycle / session_tree 改走 clear_streaming_buffers；reason 解析为 ContextTokenSettlementReason 枚举并匹配；单测
- [x] T5 回归：just fmt / lint / test / qa；llman-sdd validate c3275 --strict 绿
