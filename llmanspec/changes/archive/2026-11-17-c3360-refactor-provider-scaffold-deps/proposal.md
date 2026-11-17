---
title: provider 重复件依赖面盘点（wayfinder:research）
depends_on: []
needs_specs_change: false
branch: sdd/c3360-refactor-provider-scaffold-deps
base_branch: main
base_sha: d3a0442d85238a05a3767147e15c048a0cc449ba
---

# wayfinder:research — provider 重复件依赖面盘点

## Why

T2（provider 脚手架归属）需要逐件依赖面事实才能分件决策。已知：openai 与 anthropic 两插件 6 文件逐字节相同（trace.rs 559 / attribution.rs 177 / obs_session.rs 166 / reqwest_headers.rs 49 / dto/json_parse.rs 45 / wire_policy/defaults.rs 9，合计 1005 行/份），另有近似重复 thinking.rs（372/353）与 wire_policy/mod.rs（145/129）未逐行 diff。

## 查证问题

1. 6+2 文件的完整 use 图与外部依赖分层（fastrace / reqwest / serde_json / 插件本地类型），逐件列。
2. sdk src/obs_session.rs（104 行）与插件 provider/obs_session.rs（166 行）的关系：重复、分层还是消费关系。
3. 插件 dto::{XyChunk, XyUsage} 与 sdk 词表（消息/chunk/usage 薄留）的形状关系；llmanspec/specs/agent-llm-projection 的依赖分界条款原文。
4. hooks::HeaderBag 的定义位置与全部消费方。
5. thinking.rs 与 wire_policy/mod.rs 两份的 diff 摘要（差异行数与差异性质）。
6. hooks.rs（3 行）/ provider/tool_wire.rs（1 行）壳的调用点数。
7. 每件若迁 sdk：需带走的类型清单；若不能迁，阻塞依赖是什么。

## 产出

结论写入 llmanspec/changes/c3360-refactor-provider-scaffold-deps/research/，全部断言带路径:行号；摘要回写本 proposal Further Notes。只读代码与 specs，不改其他文件。

## Further Notes

调研完成（2026-11-18，结论全文见 research/provider-scaffold-deps.md，断言带路径:行号；主会话独立复核 obs_session facade 与 HeaderBag 定位属实）。要点：

- 逐件判定：trace.rs 可迁 sdk（带走 GenerationFinishKind、ProviderRequestTrace，无阻塞）；attribution.rs 可迁、仅 is_opencode_host 两行 reqwest::Url 例外；reqwest_headers.rs 阻塞 = reqwest 依赖；dto/json_parse.rs 阻塞 = partial-json-fixer 依赖；wire_policy 可迁（带走 ExtraPolicy、WirePolicy、defaults 4 const；openai 独有 prompt-cache retention 位置 None 即统一，行为不变）；thinking.rs 核心可迁，compat 分派参数化后 dialect 留 openai 插件。
- 修正架构审查 W1 两处：obs_session 非重复——sdk src/obs_session.rs:1-104 是唯一实现，插件份 1-7 行 re-export + 第 9 行起测试（facade，kernel 经 openai 壳消费）；XyChunk / XyUsage 只定义在 sdk（dto/chunk.rs:12、dto/message.rs:234），插件 dto/mod.rs:4 glob re-export 零差异。真实实现重复为 839 行/份（trace + attribution + reqwest_headers + json_parse + defaults），另有 wire_policy/mod 与 thinking 两份近似重复。
- 壳：hooks.rs 3 行 ×2（HeaderBag 真身 = sdk ports/http.rs:8 type alias）；provider/tool_wire.rs 1 行 ×2。删壳 = 改 10 文件 import。
- spec 分界：agent-llm-projection r1559「agent 不得直接依赖 provider-openai 的 HTTP / vendor SDK 类型」——迁 sdk 后该指称随删除清理。
- 遗留给决策票的输入：sdk 是否接受 reqwest / partial-json-fixer 两个新依赖，直接决定 attribution / reqwest_headers / json_parse 三件去向（入 T0 准则与 T2 归属判定）。
