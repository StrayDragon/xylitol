# Tasks — c2811-add-compact-model-selection

## 测试 seam（复用既有 harness，不新发明）

1. **infra/config**：compaction 配置解析单测（锚点复用 / 独立锚点 / 内联三形态、secret 渲染 × 锚点、Settings 面忽略）。
2. **agent/model**：任务级解析入口单测（解析成功独立实例、构建失败回退 + 回退态、thinking 覆盖命中 / 集外回退继承档）。
3. **llm_summarizer**：FakeModel 捕获 options（thinking_level 进请求、max_output 预算式不变、未配置形状对照）。
4. **BDD**：c7 修订 / rc15 / registry m18 可执行场景的 steps + bindings（`tests/bdd/` 既有步骤族）。
5. **obs**：provider-trace / accounting 摘要请求模型标注（既有 obs harness）。

## Tasks

### T1 配置面直通：protocol 类型 + infra 解析

- [x] `XyCompactionSettingsConfig` 新增 `model: Option<XyModelEntryConfig>` 与 `thinking_level: Option<String>`；`XyModelEntryConfig`（`protocol/model_entry.rs`）为模型条目 wire SSOT，`infra::ModelEntry` 为类型别名
- [x] infra 解析复用 SSOT（无 `XyTaskModelConfig → ModelEntry` 转换层）；Settings merge 不携带两键（settings.json 出现即忽略）
- [x] 单测：三形态解析、锚点 × secret、Settings 面忽略
- [x] `just gen-config-example` 模板同步（改 `scripts/gen_config_example.py`，跑生成，勿手改 `configs/example.yaml`）

撑 spec：`runtime-config` rc15、`domain-compaction` c14。

### T2 任务级模型解析 seam（agent）

- [x] 「按用途解析模型」单一入口：任务条目 → 注入 `XyModelBuilder` → 独立实例；缺失/解析/构建失败 → 回退当前模型 + 回退态（供通知与归因）；任务条目不注册进 ModelRegistry
- [x] thinking 解析：缺省继承（对话同款默认档：可调=末项，仅 off=off）；覆盖值精确匹配支持集，集外回退继承档 + 可观测
- [x] 单测：解析成功独立实例；构建失败回退；覆盖命中 / 集外；未声明 thinking_levels 条目继承 off

[blocked-by: T1] 撑 spec：`runtime-model-registry` m18。

### T3 摘要路径接线（compaction 三调用点）

- [x] threshold auto / overflow / manual force 三个调用点换用解析 seam（orchestrator 签名不动，仍收 `&dyn XyModel`）
- [x] `llm_summarizer` options 透传继承/覆盖后的 thinking level（含 level_map）；`max_output_tokens` 预算式不变（reserve×0.8 下限 256）
- [x] 回退通知：每 compaction 至多一次（split-turn 双摘要共享），经既有 notice 通道（含 manual 路径）
- [x] 单测（FakeModel）：thinking_level 进请求；通知至多一次；未配置时请求形状对照

[blocked-by: T2] 撑 spec：`domain-compaction` c7。

### T4 obs 归因

- [x] 摘要请求 provider-trace / accounting 标注：requested 摘要模型、actual 模型、fallback 布尔；复用 obs_session 通道
- [x] 核对 `infra-otel` / `package-ai-bridge-accounting` 既有字段；缺口则补该两面条款（specs 随实施落地）
- [x] 单测 / 既有 obs harness 验证标注

[blocked-by: T3]

### T5 BDD 场景与收口

- [x] 已落可执行场景（c7 三场景、rc15 两场景、m18 两场景）补齐 step 定义，`cargo test --lib --all-features tests::bdd::` 全绿
- [x] `just fmt` / `just lint` / `just qa` 全绿

[blocked-by: T3]
