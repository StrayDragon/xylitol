# design——权衡与裁决记录

propose 深挖裁决于 2026-09-15，事实基础均经代码核实（引用见各节）。

## D1 引用 schema：内嵌条目 + YAML anchor/alias（裁决）

三案对比：

| 案 | 形态 | 弃/取因 |
|---|---|---|
| 字符串引用 | `compaction.model: <models 键>` | 要发明一套引用解析语义；专用摘要模型必须进 models map（被 m17 注册 → 占 `--list-models`） |
| **内嵌条目（取）** | `compaction.model:` 完整对象，共享靠锚点 | YAML 原生复用，零新语法；条目不进 models map 即不注册——专用摘要模型不占模型循环（副产品收益）；未来加字符串引用是纯扩展（开闭友好） |
| 双收 | 字符串 ∨ 对象 | 两条解析路径都进合约与测试，contract 面翻倍，与「用 YAML 原生能力」的裁决理由重复 |

落地面（用户已确认）：

```yaml
# ① 复用 models 锚点（同步共享）
models:
  main: &main
    provider: openai
    model: gpt-5.2
compaction:
  model: *main

# ② 独立锚点（专用摘要模型，不进 models）
summary: &summary
  provider: openai
  model: qwen3-32b
  base_url: http://localhost:8080/v1
compaction:
  model: *summary

# ③ 直接内联
compaction:
  model:
    provider: anthropic
    model: claude-haiku-latest
```

## D2 类型落点：protocol 持 serde 形状，infra 转换，agent 只经构建器

- `XyCompactionSettingsConfig`（`src/protocol/compaction_config.rs`，c14 单一
  serde 面）新增 `model: Option<XyTaskModelConfig>` 与
  `thinking_level: Option<String>`；`XyTaskModelConfig` 为 protocol 新 serde
  类型，字段形状与 infra `ModelEntry` 兼容（serde 未知名忽略 → 锚点复用同一
  YAML 映射到两面都合法）。
- infra 侧负责 `XyTaskModelConfig → ModelEntry`（补默认）或直接装配，复用
  protocol 既有 `resolve_configured_levels` / `validate_thinking_level_map`
  （thinking 三件套校验逻辑已在 protocol，见 `ModelEntry::resolve_thinking_config`
  的 import 路径）。
- 实例构建只经注入的 `XyModelBuilder`（composition root 供；`XyModelConfig`
  是纯数据 struct，provider 构造在 agent/infra 可及处）。agent 层解析 seam
  消费「条目 → 构建器」产物，不自己 new provider。
- **Settings 面不接线**：`infra/settings/types.rs:18` 与 config.yaml 共享同一
  类型（已核实），新字段会自然渗入 Settings 反序列化——合约定为仅 config.yaml
  生效，Settings merge（`manager.rs` merge_compaction）不携带这两键，settings.json
  出现即忽略（对齐 rc12 遗留键纪律），防静默无效配置被误认生效。

## D3 thinking 继承语义（显式行为变化，已确认）

- 继承 = 所用模型的**对话同款默认档**：可调模型取支持集配置顺序末项（m10 换模
  策略同款），仅 off/不可调取 off。条目未声明 thinking_levels 时支持集仅 off
  （m9）→ 继承即 off。
- 覆盖 = `compaction.thinking_level` 精确字符串匹配所用模型支持集（m15 opaque
  纪律：不折叠大小写、不 trim）；命中 MUST 覆盖请求档位。
- 覆盖值不在支持集 → 回退继承档 + 可观测（warn + 归因标注）；MUST NOT 静默
  接受变体，MUST NOT 使整个 compaction 失败。
- 现状对照：摘要请求今天走 `XyGenerateOptions::default()`（off）；「缺省继承」
  对未配置 `compaction.model` 的用户同样生效——摘要 token 成本可能上升，提案
  已显式接受为行为变化。

## D4 回退与通知

- 回退触发面：条目缺失之外的**解析/构建失败**（含 api_key 渲染产物为空、
  provider 构建器报错）与 thinking 覆盖集外（仅档位回退继承档，非模型回退）。
- 通知纪律：每次 compaction 执行至多一次（split-turn 双摘要共享回退态，不发
  两条）；经既有 CompactionEnd notice 通道（c28 同款载荷），TUI 端呈现滚动
  提示。manual force 路径同样适用——回退告知是状态通知，不随 c28 地板诊断的
  manual 豁免。
- 归因：obs_session 通道标注摘要请求的 requested/actual 模型与 fallback 布尔；
  otel/accounting 字段缺口在 t4 实施时核对，允许补条款。

## D5 非目标（本期不做）

- 字符串别名引用（`compaction.model: cheap`）。
- slash 查看、ModelSelect 类事件、attach/remote 可见性、跨端约束板条款。
- 摘要请求 `max_output_tokens` 预算随摘要模型窗口重算（维持
  `reserve_tokens × 0.8` 下限 256；小窗口摘要模型 × 大 reserve 预算的交互记为
  已知限制，痛了再立项）。
- 触发阈值 / 切点 / c26 settlement 的窗口计量仍全部按对话模型窗口（摘要模型
  不参与触发面）。
- ModelEntry 的 `fallback` 链、`tokenizer` 字段在任务条目路径不消费（serde
  忽略，语义由 seam 层「回退当前模型」承担）。

## D6 前提与风险

- serde YAML 解析器原生支持 anchor/alias（解析层行为，非配置预处理）；配置链
  的 secret `{{ }}` 渲染在值层面，锚点复制原值后渲染仍有效——t1 以单测锁定
  「锚点复用 + secret 渲染」组合，若实现发现渲染管线在文本层先行处理则按实际
  行为收敛测试（锚点语义不受影响）。
- `XyModelConfig` 无 serde derive（纯数据 struct，已核实）——不直接内嵌它，
  走新 protocol serde 类型，避免给运行时边界类型强加 wire 职责。
