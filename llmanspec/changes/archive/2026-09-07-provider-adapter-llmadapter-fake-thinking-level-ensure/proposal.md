---
depends_on: []
rules_edit_acked: true
branch: sdd/provider-adapter-llmadapter-fake-thinking-level-ensure
base_sha: 4ab4e5cbcd96e39ee7d3101e103482d85a8fc995
checkpointed: true
checkpoint_sha: 4ab4e5cbcd96e39ee7d3101e103482d85a8fc995
---

# Provider 适配层单层化：LlmAdapter 塌缩、fake 双轨收拢、thinking 词表常量化与 ensure_downlink 拆臂

## Why

2026-09 二轮代码体检（死码 / slop / 测试盲区三线审计，落地为 e93f81ae..f01f3ce6 四轮清淤提交）把一批「疑似项」留档待正式 triage。本 change 收编其中 id 圈定的四项 infra/provider 侧结构性清欠：它们共享同一条主线——**消除单一实现的平行包装层与散落词表**，且互相咬合（fake 收拢依赖塌缩后的单层外壳），适合同分支一次落地、一次回归。风险更高的行为面项（XyEvent 补发射、navigate_tree 退役）与跨面重构（ChoicePrompt / cli::run 深拆）不在本单，见「范围外」。

## What Changes

- **LlmAdapter → AdapterXyModel 塌缩（单层适配外壳）**：`src/infra/provider/adapter/mod.rs` 的 `LlmAdapter` trait 现仅剩 `MappedBridgeAdapter` 一个实现、唯一消费者 `AdapterXyModel`，属「已是统一口再包一层」。删除 `LlmAdapter` / `MappedBridgeAdapter` / `AdapterRef`，`AdapterXyModel` 直接持有 bridge `AiBridgeLlmAdapter` 引用，经既有 `infra/provider/map.rs` 完成 DTO→XyStream 映射。真 seam 是 `XyModel`（业务唯一依赖，不变）；`pa7`（vendor 类型不出 infra/agent）与 `map.rs` 边界职责均不受影响。
- **Fake 模型双轨收拢（并入统一装配路径）**：bridge `AiBridgeModel` trait 单实现（`FakeProvider`）单消费者，签名与 `AiBridgeLlmAdapter` 异构（带 `stream: bool`、弃 `options`）。bridge 侧 `FakeProvider` 改实现 `AiBridgeLlmAdapter`（`stream` 布尔由 `generate` / `generate_stream` 双方法承载，`options` 维持忽略），删除 `AiBridgeModel` 平行 trait；主仓侧 `src/infra/provider/fake.rs` 的新 type `XyModel` 直实现退役，fake 经塌缩后的 `AdapterXyModel` 单层外壳暴露，与 vendor 同一条装配路径。对外契约不变：fake 仍是 `ScenarioStep` 驱动的确定性 `XyModel`。
- **thinking-level 内置词表常量化**：bridge `canonical_known_level` 与 Anthropic 预算表的内置档名词表（off/minimal/low/medium/high/xhigh/max）无单一来源；主仓 `protocol/model/thinking.rs` 已有 `THINKING_OFF` 与帮手但未覆盖内置全集。bridge 暴露内置档位常量表，protocol 再导出（protocol MAY 依赖 bridge DTO 约定），残留散落字面量随动。**只做常量，不做封闭枚举**：运行时档名保持 opaque 字符串（runtime-model-registry 规格钉死精确匹配、禁封闭超集）。
- **`ensure_downlink` 拆臂**：`src/app/core/driver/remote.rs` 的 `ensure_downlink`（≈240 行）按下行相位拆为臂函数（连接建立 / 订阅重试退避 / 消息分派臂），共享上下文收拢为一个持有 clone 字段的 context 结构；纯机械重排，行为与既有 downlink generation / backoff 语义（ath41/ath42/ath44）零变化。

## Capabilities

- `infra-provider`：重写 `@req:pa1`（adapter-trait → 单层适配外壳不变量；`rules_edit_acked` 已声明）。`pa6`（单一装配路径）、`pa7`（vendor 边界）语义不变、继续成立。
- `test-fake-provider`：`@req:r38` 措辞随动（「实现 XyModel」→「经统一装配路径暴露为 XyModel」；`ScenarioStep` 契约原样保留）。
- `package-ai-bridge`：`FakeProvider` 实现的 trait 更换与词表常量暴露属包内实现细节，无 live 合约钉 `AiBridgeModel`，不改 spec。

## Impact

- 受影响代码：`src/infra/provider/adapter/`（mod / xy_model / factory）、`src/infra/provider/fake.rs`、`packages/xylitol-ai-bridge/src/fake/mod.rs`、`packages/xylitol-ai-bridge/src/thinking.rs`、`src/app/core/driver/remote.rs`，及引用随动（`tests/provider_http_stream_abort.rs`、`src/agent/compaction/mod.rs`、`tests/bdd/steps_domain_compaction_extra.rs`、3 个 `lab_*` examples 与 1 个 lab test）。
- 行为变化：无（四项均为结构收拢 / 常量化 / 机械拆分；fake 的 options 忽略、stream 语义、downlink 重连语义全部保持）。
- 测试：复用既有单测与 BDD seam，不新扩 BDD step，不打网。

## 范围外（篮中剩余，后续拆单）

原清欠草案中以下项**不在本单**，留待各自独立 change：`ChoicePrompt::render` 与 `cli::run` 深拆（跨面重构，独立回归面）；`navigate_tree` 退役（需动 `agent-session-store` live 场景 + steps，成组退役）；`XyEvent` 未发射变体补发射（行为变更，方向已定：补发射不退役，设计点为状态归属与事件去重）。
