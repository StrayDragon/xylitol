---
title: provider 脚手架归属决策（wayfinder:explore）
depends_on:
  [
    c3355-refactor-sdk-inclusion-boundary,
    c3360-refactor-provider-scaffold-deps
  ]
needs_specs_change: false
branch: sdd/c3375-refactor-provider-scaffold-home
base_branch: main
base_sha: a638ea4b9fa8ed0036a068fd0ad9ba593c372f7a
---

# wayfinder:explore — provider 脚手架归属决策

## Why

openai 与 anthropic 两插件 6 文件逐字节相同（1005 行/份：trace / attribution / obs_session / reqwest_headers / json_parse / wire_policy defaults），修一个 bug 须手工镜像两份；另有近似重复 thinking.rs 与 wire_policy/mod.rs 待 R1 定性。hooks.rs（3 行）/ tool_wire.rs（1 行）纯再导出壳。

## 待决策问题

1. 分件归属：纯工具件（json_parse、wire_policy defaults）、fastrace 件（trace）、reqwest 件（reqwest_headers）、HeaderBag/attribution/obs_session 各落哪里——依 T0 准则逐件判定。
2. T0 准则判「不得入 sdk」的件：留在插件并显式记录「接受镜像」为决策，还是其他落点。
3. dto::{XyChunk, XyUsage} 耦合的处置（依 R1 结论）。
4. 新方言插件的脚手架成本基线：收敛后新插件要写多少行才有一个可 trace 的方言。
5. 实施路径判定：propose 还是 quick；是否拆多张。

## 事实底账

- 6 文件 diff 全 0（2026-11-17 实测）；依赖面分层初判：json_parse 仅 serde_json；trace 依赖 fastrace + 插件 dto；attribution 依赖 HeaderBag + obs_session；reqwest_headers 依赖 reqwest。
- ADR 对照：c2833 one-path-one-root 不涉脚手架归属；src/AGENTS「Provider 与消息」的 sdk 边界由 T0 仲裁。

## 完成判据

已达成（2026-11-18）：每件有归属结论与理由（Q3 清单）；实施路径明确（随 T5 收拢）；R1 的 7 个查证问题全部被答案覆盖。

## Open Questions（逐问深挖记录）

- **Q1 thinking 收敛深度（已定 2026-11-18）：随本票收敛。** sdk 持 thinking 核心（ThinkingAdapterKind / ResolvedThinking / BuiltinThinkingLevels + resolve/apply），compat 分派点参数化（注入点由插件提供），dialect 留 openai 插件。消除最大近似重复 372/353 行；差异是方言分派非漂移（c3360 research）。
- **Q2 is_opencode_host（2026-11-18 提出，被 Q3 取代）：** 原拟「sdk 归因函数收 opencode 判定入参、判定留插件」；Q3 收缩版改判 attribution 整件留镜像后，sdk 无归因函数，参数化注入随之不需要。此条仅存档。
- **Q3 分件归属总清单（已定 2026-11-18，收缩版；容器 = sdk `provider/` 单一领域模块，对齐 mcp.rs / loop_middleware.rs 先例）：**
  - **迁 sdk `provider/`**：trace.rs（559×2 全 pub 面：GenerationFinishKind / ProviderRequestTrace + 7 方法 / Drop）；wire_policy（mod 145/129 + defaults 9×2：ExtraPolicy / WirePolicy 六方法 / defaults 4 const；openai retention 位置 None 统一）；thinking 核心（372/353：ThinkingAdapterKind / ResolvedThinking / BuiltinThinkingLevels + resolve_thinking_for_request / apply_thinking_* 四函数 / resolve_from_options；compat 分派参数化，dialect 留 openai）；dto/json_parse.rs（45×2，partial-json-fixer 个案准入）；**AdapterKind / AdapterRef**（追加件：kernel adapter/mod.rs:5 实测跨方言，anthropic-messages 同枚举映射，anthropic thinking.rs:8 同构在用）。
  - **留插件镜像（显式决策）**：attribution.rs 整件（177×2，API 卫生——opencode courtesy 是网关私事，第三方方言作者不需要）；reqwest_headers.rs（49×2，传输栈一票否决）；is_opencode_host 随 attribution 留镜像。
  - **不动**：obs_session（facade，c3360 修正）；dialect/ 留 openai；adapter 工厂（build_adapter_with_wire_policy）留 openai 插件、改吃 sdk 类型，kernel→openai 插件工厂边保留。
  - **删壳**：hooks.rs（3 行×2）/ tool_wire.rs（1 行×2），调用点直连 sdk（约 10 文件 import 改动）。
  - **kernel 消费面**：trace / WirePolicy / Compat / AdapterKind 改直连 sdk（不再经 openai 插件内部路径）。
  - **新方言成本基线**：脚手架复制从 ~1005+ 行/方言降为 0（trace/wire_policy/thinking/json_parse 来自 sdk）；仅网关 courtesy 需要时可选复制 attribution + reqwest_headers（约 226 行）。
  - **张力消歧**：src/AGENTS.md「Provider 与消息」节「只收词表薄留」句随实施 propose 一行改述为「词表薄留 + 收纳准则所列共享件」。
  - **实施路径**：本票止于决策；实施随地图 T5 分组决策收拢（倾向与其他票合并一张 propose）。

## Further Notes

（决策与 research 结论回写处）
