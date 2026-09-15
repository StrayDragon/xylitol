# tasks——垂直切片

测试 seam 已确认（propose 2026-09-15）：五个 seam 全部复用既有 harness
（infra config 单测、agent model 单测、llm_summarizer FakeModel、rstest-bdd
runner、obs harness），不发明新 seam。

## t1 配置面直通（protocol + infra）

- `XyCompactionSettingsConfig` 新增 `model: Option<XyTaskModelConfig>` 与
  `thinking_level: Option<String>`；`XyTaskModelConfig` protocol serde 类型
  （字段与 infra ModelEntry 形状兼容，serde 未知名忽略）。
- infra 解析/转换（含 `XyTaskModelConfig → ModelEntry` 或直接装配路径）；
  Settings merge 不携带两键（settings.json 出现即忽略）。
- 单测：锚点复用解析（models 锚点 alias 进 compaction.model）、独立锚点、
  内联三形态；secret `{{ }}` × 锚点组合；Settings 面忽略。
- `just gen-config-example` 模板同步（改 `scripts/gen_config_example.py`，
  跑生成，勿手改 `configs/example.yaml`）。

## t2 任务级模型解析 seam（agent）

- 「按用途解析模型」单一入口：任务条目 → 注入 `XyModelBuilder` → 独立实例；
  缺失/解析/构建失败 → 回退当前模型 + 回退态（供通知与归因）。
- thinking 解析：缺省继承（对话同款默认档：可调=末项，仅 off=off）；覆盖值
  精确匹配支持集，集外回退继承档 + 可观测。
- 单测：解析成功（实例独立于当前模型）；条目构建失败回退 + 回退态置位；
  覆盖命中 / 覆盖集外回退继承档；未声明 thinking_levels 条目继承 off。
- [blocked-by: t1]

## t3 摘要路径接线（compaction 三调用点）

- threshold auto / overflow / manual force 三个调用点换用解析 seam（orchestrator
  签名不动，仍收 `&dyn XyModel`）。
- `llm_summarizer` options 透传继承/覆盖后的 thinking level（含 level_map）；
  `max_output_tokens` 预算式不变（reserve×0.8 下限 256）。
- 回退通知：每 compaction 至多一次，经既有 notice 通道（含 manual 路径）。
- 单测（FakeModel）：捕获 options.thinking_level 进请求；回退通知至多一次
  （split-turn 双摘要共享）；未配置时与现状请求形状一致（除 thinking 继承）。
- [blocked-by: t2]

## t4 obs 归因

- 摘要请求 provider-trace / accounting 标注：requested 摘要模型、actual 模型、
  fallback 布尔；复用 obs_session 通道。
- 核对 `infra-otel` / `package-ai-bridge-accounting` 既有字段；缺口则补该两面
  条款（specs 随实施落地）。
- 单测/既有 obs harness 验证标注。
- [blocked-by: t3]

## t5 BDD 场景与收口

- Specs landing 已落的可执行场景（c7 修订、rc15、registry 新规）补齐 step
  定义，`cargo test --lib --all-features tests::bdd::` 全绿。
- `just fmt` / `just lint` / `just qa` 全绿。
- [blocked-by: t3]
