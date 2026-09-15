---
depends_on: []
---

# compact 摘要请求的模型与 thinking 可配——缺省继承，配置覆盖，任务级解析 seam

## Why

compaction 的摘要请求目前**固定复用当前会话模型且固定 thinking off**（三条路径
threshold auto / overflow / manual 都经 `build_current_model` 取当前模型；摘要请求
`XyGenerateOptions` 走 default，即 off。见 `src/agent/capabilities/compact_ops.rs`、
`src/agent/runtime/react/turn_end.rs`、`src/agent/compaction/llm_summarizer.rs`）。
用户实测场景（c2810 的动因形态：本地量化模型刻意以 32k 小窗口运行 256k 大模型）
暴露出摘要请求绑定的结构性问题：

- **摘要任务与对话任务的需求不同质**：摘要是结构化抽取（Goal/Progress/Next
  Steps 骨架），外部 harness 常见做法是摘要走更快、更便宜的模型（本地小模型 /
  廉价云端模型），对话继续用主力模型。
- **模型与 thinking 选择权被 harness 收走**：用户能选「用什么模型、什么档位对话」，
  却不能选「用什么模型、什么档位压缩」——compaction 请求与正常用户 turn 对话是
  同等地位的 API 调用，选择权应还给用户。
- 小窗口场景下摘要请求本身也受窗口约束，专用小模型往往与窗口预算更匹配。

c2810（已归档）已让摘要请求成为**独立请求且带独立参数**（`max_output_tokens`
接线），模型与 thinking 独立是其自然延伸。

## What Changes

（已裁决，2026-09-15 propose 深挖确认）

- **配置面**：`compaction.model` 新增可选字段，接受**完整条目对象**（字段形状与
  `models` 条目兼容：provider/model/base_url/api/compat/api_key/context_window/
  thinking 三件套）；跨条目共享靠 **YAML anchor/alias 原生能力**，不引入字符串
  别名引用语义。配套 `compaction.thinking_level` 可选档名覆盖。两个字段**仅在
  config.yaml 加载面生效**；Settings.json 的 compaction 块不接线（出现即忽略，
  对齐 rc12 遗留键纪律）。
- **任务级模型解析 seam**：落点不是「compact 特判」，而是「按用途解析模型」的
  单一入口（任务模型条目 → 经注入的 provider 构建器产出独立实例）；compact 摘要
  是第一个消费者，后续同类旁路任务（branch summary、title 生成等）复用同一入口，
  不预建未落地任务的抽象。任务条目**不注册**进 ModelRegistry——专用摘要模型
  不占 `--list-models` 与模型循环。
- **解析与回退语义**：未配置 `compaction.model` 时摘要用当前会话模型（模型继承
  零变化）；条目解析或构建失败时 MUST 回退当前模型，MUST NOT 静默换模型。
- **thinking 语义**（显式行为变化）：摘要请求 thinking 缺省**继承**所用模型的
  对话同款默认档（可调模型=支持集末项，仅 off/不可调=off；与 m10 换模策略同款）；
  `compaction.thinking_level` 显式配置且精确匹配所用模型支持集时 MUST 覆盖；
  覆盖值不在支持集时 MUST 回退继承档并可观测，MUST NOT 静默接受变体（m15
  opaque 纪律）。今天摘要请求固定 off，缺省继承对未配置用户也是行为变化——
  已显式接受（摘要 token 成本可能上升）。
- **回退可见性**：obs/provider-trace 归因始终记录摘要请求实际用模与 fallback
  形态；回退发生时经既有通知通道提示，每次 compaction 执行至多一次（history +
  turn-prefix 双摘要共享回退态；对齐 c28 通知作用域纪律；manual 路径同样适用）。
- **交互面**：仅配置文件。slash 查看、ModelSelect 类事件、attach/remote 客户端
  可见性全部不做。

## Capabilities（核对后）

- `runtime-config`：rc15（compaction 配置映射）字段扩展——model 内嵌条目
  （锚点复用语义）+ thinkingLevel；Settings 面不接线条款。
- `runtime-model-registry`：新增规则——任务级模型解析入口（解析→构建→回退
  可观测；不注册进 registry）。
- `domain-compaction`：c7（LLM 摘要——模型/thinking/回退语义修订）、c14
  （单一配置来源字段列表扩展）。
- `agent-runtime`：摘要路径接线核对（turn-end / overflow / manual 三调用点
  换用解析 seam；既有 ar 编排条款预计不动）。
- `infra-otel` / `package-ai-bridge-accounting`：摘要请求模型归因标注（复用
  obs_session 通道；具体字段缺口在实施时核对）。

## Impact

- 行为合约变更（c7/c14/rc15/registry 新规）→ 完整 SDD pipeline（本提案）。
- 兼容红线（改写后）：未配置 `compaction.model` 时模型继承当前（零变化）；
  thinking 缺省从固定 off 变为继承（显式裁决的行为变化）；会话 JSONL 与
  compaction 条目形状不变。
- 非破坏性合约变更：纯新增可选字段，无移除/重命名 → 不需要 `migrations/`。
- 非目标：字符串别名引用；摘要请求窗口预算随摘要模型窗口重算（预算式维持
  reserve 派生，`reserve_tokens × 0.8` 下限 256 不变）；触发阈值/切点仍按对话
  模型窗口计量；跨端约束板新条款（本期无端侧差异面）。
- 配置示例经 `just gen-config-example` 同步（改 `scripts/gen_config_example.py`
  模板，勿手改产物）。
