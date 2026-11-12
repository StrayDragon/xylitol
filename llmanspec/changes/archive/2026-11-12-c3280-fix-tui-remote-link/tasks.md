# Tasks — TUI 远程驱动链路状态修复

> 实施者导读：design §7 是复核结论。收口走 finalize，不列为任务。

- [x] T1 spec delta：app-tui-host r3330 链路相位、r3331 attach observe、r3332 turn cancel、r3333 历史提示、r3334 Esc 窗口、r3335 peer 状态
- [x] T2 LinkPhase：remote.rs 单字段相位替换 subscribed_ok/resync_needed/fatal；link_health 派生（仅 Live=Up）；fatal 可观测 + prompt 提交触发重试；单测
- [x] T3 PeerState：http_ws.rs closed/last_err/dead 合一 + Notify；reset_mux/ensure_peer 换 peer 清原因；单测
- [x] T4 turn_cancel：AgentEnd / 流关闭复位新 token；单测
- [x] T5 AttachObserve：Ended 仅经显式 resubscribe 回 Live；单测
- [x] T6 历史窗口与 Esc：history_notice 显式字段 + 文案定位；EscWindow 归 host 单点；单测
- [x] T7 回归：just fmt / lint / test / qa；llman-sdd validate c3280 --strict 绿
