---
title: Command 到 Outcome 翻译合约归属（wayfinder:explore）
depends_on: [ c3355-refactor-sdk-inclusion-boundary, c3370-refactor-dispatch-twins-shape ]
needs_specs_change: false
branch: sdd/c3385-refactor-dispatch-contract-home
base_branch: main
base_sha: d881e012ede75847771d8eb52698a172a99fd373
---

# wayfinder:explore — Command→Outcome 翻译合约归属

## Why

「哪些 Command 可执行、哪些 transport-only、产出什么 Outcome」写在三处：app-tui remote.rs（104 处 Command:: 引用）、harness.rs ScriptedDriver（51 处，本地再实现产品语义）、dispatch.rs 测试桩；transport-only 11 变体名单逐字双份（remote.rs:1660-1670 = harness.rs:1336-1346）。跨 crate 层面 kernel 与 app-tui 各有一份 dispatch 双胞胎（含 validate_nonempty_thinking_level 双副本，kernel 侧为生产路径）。新增 Command 现需改 3 处 + 2 份名单。

## 待决策问题

1. 共享层级：protocol / plugin-sdk / 各自保留——依 T0 准则与 R3 的形状对比；错误类型抽象的最小面。
2. transport-only 判定与（可选）按变体 outcome 构造收敛到单点的形态；remote 只写传输、Scripted 只写剧本注入的边界。
3. app-tui 版 SessionCommandExecutor supertrait（6 行，crate 内无独立 dyn 消费者）是否随本决策一并并回（审查 W14）。
4. kernel 侧 dispatch 是否同轮收敛，还是仅消除 app-tui 内部三写。
5. 实施路径判定：propose 还是 quick。

## 完成判据

翻译合约单一真值点的落点与形状定稿；新增 Command 的改动点数有明确预期（从 3+2 降到多少）；实施路径明确。

## Open Questions（逐问深挖记录）

- **Q1 收敛范围（已定 2026-11-18）：kernel 同轮收敛。** kernel 删本地 DispatchOutcome / validate / transport_variant_error / variant_name / SessionCommandExecutor，全改 import sdk；command_exec.rs impl 改实现 sdk trait；kernel 只留独有 outcome_to_value（RPC 结果序列化）。双胞胎从根上消失，新增 Command 从 3+2 处降 1+1（protocol 加变体 + sdk 单一 match）。回归面 = kernel dispatch + occupancy + BDD server 群。
- **Q2 错误抽象最小面（已定 2026-11-18）：sdk 级 DispatchRejection + 两端 From wrap。** sdk 定义极小拒绝类型 DispatchRejection { TransportOnly { method }, EmptyThinkingLevel }（命名实施定）；合约函数返回 Result<DispatchOutcome, DispatchRejection>；两端各自 impl From 到既有 XyDriverError，flatten / log_failure / detail_kind 行为零变化。XyDriverError 本体留两端（三层契约「② 应用协议 → runtime」不动；runtime 独有 From<RuntimeControlError/HookBlockedError> 与 app-tui 独有 From<ClipboardError> 是本质差异，不强行统一），sdk 保持零重依赖。否决整错误上移（撞分层）与泛型误差回调（冗长无益）。
- **Q3 收尾打包（已定 2026-11-18，确认）：** ①数据类型下沉 sdk 5 件——ModelInfo / SessionState / CommandInfo / RuntimeReloadReport / LoadedResourcesSnapshot（SessionListEntry 已在），outcome 变体引用 sdk 类型，Command 经 sdk→protocol 边引用；②W14 并回——app-tui 本地 SessionCommandExecutor（6 行无独立 dyn 消费者）删除，sdk trait 为两端唯一 trait、app-tui XyDriver 以其为 supertrait；③三写边界——本质不合并（kernel 直调 runtime / remote unary RPC+JSON / harness 剧本注入），漂移全收敛（transport-only 名单三份→sdk 单点；4 变体直映射→sdk 构造 helper；fork position / session_path 解析双拼写→sdk 共享 helper）；④改动点预期——新增 Command 从 3+2 处降 1+1；⑤路径——独立 propose（sdk + Cargo sdk→protocol 边 + kernel 瘦身换绑 + app-tui 三写收敛），spec 预期零变更，回归面 = kernel dispatch + occupancy + BDD server 群 + app-tui harness 群，是否并入大 propose 由 T5 裁。

## 完成判据

已达成（2026-11-18）：翻译合约单一真值点落点（sdk）与形状定稿（Q1/Q2/Q3）；新增 Command 改动点预期明确（1+1）；实施路径明确（独立 propose）。

## Further Notes

（决策与 research 结论回写处）
