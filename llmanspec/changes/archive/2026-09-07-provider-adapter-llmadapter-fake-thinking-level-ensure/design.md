# Design: Provider 适配层单层化

四个决策点，均已在 recon（代码事实 + 既有规格约束）中落定；实现按 tasks 展开，不再重开。

## D1 LlmAdapter trait：塌缩，不保留

- **事实**：e93f81ae 删掉三个零调用包装壳后，`LlmAdapter` 仅剩 `MappedBridgeAdapter` 一个实现，唯一消费者 `AdapterXyModel`；1:1 转发 `generate` / `generate_stream`。
- **裁决**：塌缩。依据 `src/AGENTS.md`「扩展开闭」实现优先序——新 port 仅在「真实第二实现或嵌入/测试必须替换时」；禁止「已是统一口再包一层」。真 seam 是 `XyModel`（agent / capabilities 只认它）；`LlmAdapter` 不是 `Xy*` 跨层契约，属 infra 内部细节。
- **否决案**：「第二实现出现前保留 trait」。预留一个无多态价值的空层，违背 Pre-0.0.1 卫生精神；且塌缩后若真出现第二 adapter 来源，加回 trait 是一次普通的提层重构，成本不高于现在保留它的持续阅读税。
- **边界不变量**：映射函数留在 `infra/provider/map.rs`（「真边界差归 infra map」），`AdapterXyModel` 只内联**调用**；`pa7`（vendor 类型不出 infra/agent）继续成立。

## D2 fake 走 AiBridgeLlmAdapter，删 AiBridgeModel

- **事实**：bridge `AiBridgeModel` 单实现（`FakeProvider`）单消费者（主仓 fake 新 type 的 XyModel 直实现），签名与 `AiBridgeLlmAdapter` 异构（`stream: bool` 参数、忽略 options）。vendor 三 adapter 走 `MappedBridgeAdapter` 映射，fake 手写同构映射——同一 DTO 映射两条平行路径。
- **裁决**：bridge `FakeProvider` 改实现 `AiBridgeLlmAdapter`：`stream: bool` 由 `generate`（false）/ `generate_stream`（true）双方法承载；`options` 维持忽略（现行为）。删除 `AiBridgeModel`。主仓 fake 新 type 退役，改为 re-export bridge 类型 + 经 `AdapterXyModel` 构造，与 vendor 同一条装配路径（`pa6` 的「单一装配路径」由此覆盖 fake）。
- **否决案**：「保留 AiBridgeModel、仅共享 map 函数」——留着一个语义异构的平行 trait，双轨未收拢，只是去重。
- **行为保持承诺**：`ScenarioStep` 序列、delay、错误注入、`stream` 语义、options 忽略全部不变；`test-fake-provider` 全部既有测试零断言改动通过（构造调用点随动除外）。

## D3 词表形态：常量表，不做封闭枚举

- **硬约束**：runtime-model-registry 规格钉死档名为 opaque 字符串、精确匹配、`MUST NOT` 要求属于封闭枚举超集、`MUST NOT` 静默展开全球超集。任何 `enum ThinkingLevel` 都违约。
- **裁决**：bridge `thinking.rs` 暴露内置档位常量表（`off/minimal/low/medium/high/xhigh/max`），`canonical_known_level` 与 Anthropic 预算表引用之；`protocol/model/thinking.rs` 再导出（protocol MAY 依赖 bridge DTO 约定）。主仓残留字面量（dispatch 测试档位表等）改引常量。新档名进表 = 一次表编辑，无类型分叉。
- **归属理由**：词表家在 bridge——预算映射与请求组装语义同处；protocol 只做再导出，避免主仓 / bridge 两表漂移。

## D4 ensure_downlink 拆臂：按相位拆自由函数 + context 结构

- **事实**：≈240 行单函数，内联了连接建立、订阅重试退避（ath42 静默）、消息分派多臂、generation 丢弃（ath41）、backoff 升级（ath44-flapping），开头 clone 约 18 个共享字段。
- **裁决**：共享字段收拢为一个 downlink context 结构（一次构造，臂函数按引用取用），按相位拆臂函数（connect / subscribe-retry / message-dispatch 主臂），消息分派臂内再按 `RpcMessage` 类型保留既有 match 结构。行为零变化，注释（ath41/ath42/ath44/c2480）随代码搬迁不重写。
- **否决案**：「按行数对半拆」——逼出假拆分；「引入 trait 抽象 driver 臂」——为单一实现造层，违反 D1 同款裁决。
- **验收**：remote.rs 既有内联测试（重连、generation 丢弃、backoff、subscribe 失败静默等）零断言改动全绿。

## 风险与回归面

- t2（塌缩）与 t3（fake）触碰同一批文件，**必须串行**：先塌缩出单层外壳，fake 再并入，避免中间态双轨三义。
- t4（词表）与 t5（拆臂）与其余切片无文件交集，可并行推进。
- 全程无行为变化断言改动；若任何既有测试需要改断言才能过，即视为实现错误而非测试过期（本 change 无合约行为变更）。
