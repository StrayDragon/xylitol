# c2827 执行纪要（对计划的偏离与证据）

## pending 对账
- 基线 477（62 cap）→ 收口 401（60 cap，infra-network 整系删除后 63，两 cap 清零不删目录则 60）。
- 削减 76 = 转场景 68 + 移除 10（r31、r1089、r1754、r1757、r1455–r1460×6）+ 合并 4（r1758、r1756、r1768、r1769）。
- 目标差：计划估 373（全批准移除时），实际 401，差 +28 来自执行中降级（见下）。

## 执行中降级 / 重分类（均带证据，规范行为不变）
- (a) 等价单测已有：r1040（test_persist_done_usage）、r1057（test_set_tools_takes_effect_on_next_turn）。
- (b) 跨能力场景已覆盖：r1005（test-hooks-wiring model-select/thinking-select）、r1745（cli-entry zero-model-config-hard-fails）、r1744 与 r1498（package-ai-bridge r1538/r1539 场景逐字重复，不重复挂）。
- (c) 缺断言面（点名缺口）：
  - r1462/r1473/r1474：llm.request 仅由 native HTTP 适配层导出，fake 无 HTTP 层（steps_otel_obs.rs:238 注释即证）——需 native mock 流 harness。
  - r1475（并行窗+otel 跨 harness 组合）、r1488（file+OTLP 双 reporter 装配缝）、r1492（fake Done 恒 usage:None）。
  - r1415（压后地板判定内联于 orchestrator 压后收尾，需满窗注入场景）。
  - r1790（session/resources 下行需真 server 客户端循环缝）。
  - r1042（两轮间换模的第二轮请求绑定无按模型记录缝）、r1122（XyToolCtx state_events 无生产构造方，TodoUpdated 事件链未接线）。
  - r1316（editor seed job 仅生产 run loop 应用，pump 不执行）、r1336（ScriptedDriver 返回样例树，生产树构建不可达）。
  - r1375（trust 闸需真 TTY TerminalGuard）。
  - r1262 部分：空输入不提交已断言；滚底子句由包级引擎单测承载。
- 新移除候选（待用户裁决）：cli-entry r67/r68——SlashCommandInfo/SlashCommandSource{prompt|skill} 类型全仓零实现，且与 pt3（prompts 不再发现）冲突；已在 feature 内注释标记。

## 技术修正记录
- ScriptedDriver get_messages 失败旋钮必须挂在 dispatch(Command::GetMessages) 臂（session_entries 走 dispatch 而非 get_messages 方法）。
- fastrace 后台批量投递会跨 SpanCollectScope 泄漏 span：计数型断言一律收窄到本会话/本 trace（沿用第一波 r1479 修复口径）。
- otel/obs 新绑定全部 #[serial]（全局观测闸与收集槽为进程级）。
