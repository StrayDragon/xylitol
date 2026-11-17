---
title: kernel 与 app-tui dispatch 双胞胎形状对比（wayfinder:research）
depends_on: []
needs_specs_change: false
branch: sdd/c3370-refactor-dispatch-twins-shape
base_branch: main
base_sha: b2a69f4fef9b4a789b2a071d4d711e8eb46cd9f4
---

# wayfinder:research — kernel↔app-tui dispatch 双胞胎形状对比

## Why

T4（Command→Outcome 翻译合约归属）需要两份 dispatch 的全量差异与共享约束。已知：kernel src/dispatch.rs（563 行）与 app-tui src/dispatch.rs（262 行）各自定义 DispatchOutcome、transport_variant_error、validate_nonempty_thinking_level；两枚 DispatchOutcome 挂在不同源的 XyDriverError 上（runtime 侧 vs app-tui driver/proto 客户端侧）；app-tui 内部三写（remote.rs 1582-1671 / harness.rs ScriptedDriver 1003-1347 / dispatch.rs 测试桩），transport-only 11 变体名单在 remote.rs:1660-1670 与 harness.rs:1336-1346 逐字双份。

## 查证问题

1. 两份 DispatchOutcome 逐变体差异；两枚 XyDriverError 的类型源与差异面。
2. transport_variant_error / validate_nonempty_thinking_level 两份副本的 diff（kernel 侧为生产路径：dispatch.rs:261、driver/command_exec.rs:180）。
3. Command 类型归属（xylitol-protocol？）；两端已有的共享先例（plan_message_history_travel 经 plugin-sdk）。
4. app-tui 内部三写中哪些差异是本质（传输 vs 剧本注入）、哪些是漂移。
5. 若建共享翻译合约：候选落点（protocol / plugin-sdk / 各自保留）各自需要的形状与错误类型抽象最小面。

## 产出

结论写入 llmanspec/changes/c3370-refactor-dispatch-twins-shape/research/，全部断言带路径:行号；摘要回写本 proposal Further Notes。

## Further Notes

调研完成（2026-11-18，结论全文见 research/dispatch-twins-shape.md，diff -u 实测）。要点：

- 双胞胎形状：两份 DispatchOutcome 25 变体逐字零差异（kernel dispatch.rs:22-60 == app-tui dispatch.rs:22-60）；差异仅 4 处 hunk——错误导入路径、kernel 独有 outcome_to_value 序列化层（kernel dispatch.rs:62-160）、dispatch() 接收者分叉（kernel:176 dyn SessionCommandExecutor vs app-tui:76 dyn XyDriver，后者以前者为 supertrait）、import 排版。transport_variant_error / validate_nonempty_thinking_level / variant_name 三函数逐字相同；生产校验调用在 kernel driver/command_exec.rs:180（本票 Why 原引 dispatch.rs:261 为测试桩，以此为准）。
- 两枚 XyDriverError：kernel 侧 = runtime/driver/error.rs:11 单一定义，app-tui 侧 = 本地 driver_error.rs:10；7 变体 + from_opaque + 8 个共享 From 逐字相同、源类型全在 sdk::error；差异仅 runtime 独有 From<RuntimeControlError/HookBlockedError> 与 app-tui 独有 From<ClipboardError>。
- 归属事实：Command 在 xylitol-protocol/src/command.rs:8；protocol 与 plugin-sdk 互不依赖；runtime 不依赖 protocol、无 execute_session_command——翻译层只在 kernel 与 app-tui，两者均已依赖 sdk + protocol。sdk 跨端先例 4 条（plan_message_history_travel 双消费、SessionListEntry、CompactionError、XyError 词表族）。
- 三写判定：本质不可合并 = kernel 直调 runtime 能力 / remote unary RPC + JSON 重组 / harness 剧本注入（错误注入、travel_overrides）；可合并漂移 = transport-only 11 变体名单实为**三份逐字**（kernel driver/command_exec.rs:81-91、remote.rs:1683-1693、harness.rs:1335-1345；本票 Why 原记双份，以此为准）、4 变体直映射三份、fork position 与 session_path 解析双拼写、stats JSON 形状双三份。同目录 types.rs / proto.rs 已发生「放任复制→实质分叉」先例。
- 候选落点结论：protocol 不可行（payload 全类型 + 错误进信封 crate，或拉 rutis/tokio 重依赖，违信封定位）；**推荐 plugin-sdk**——最小面 = ①新依赖边 sdk→protocol（无环）②下沉 5 个数据类型（ModelInfo / SessionState / CommandInfo / RuntimeReloadReport / LoadedResourcesSnapshot）③错误只 kind 化构造、XyDriverError 本体留两端 wrap ④翻译合约本体（DispatchOutcome / SessionCommandExecutor / dispatch() / validate / transport_variant_error / variant_name）进 sdk ⑤kernel 留 outcome_to_value，两端依赖集零变化。各自保留 = 每新增 transport-only 变体同步 3 处 tail，分叉轨迹已实测。
- 行号漂移提醒：本票 Why 所引 remote.rs / harness.rs 行号为审查时值，现值 remote.rs:1601-1696 / harness.rs:1001-1348。
