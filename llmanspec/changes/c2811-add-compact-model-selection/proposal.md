---
depends_on: []
---

# compact 摘要模型可选——默认继承当前模型，任务级模型选择可扩展

## Why

compaction 的摘要请求目前**固定复用当前会话模型**（turn-end / force 路径都经
`build_current_model` 取当前模型；见 `src/agent/runtime/react/turn_end.rs` 与
`src/agent/capabilities/compact_ops.rs`）。用户实测场景（c2810 的动因形态：
本地量化模型刻意以 32k 小窗口运行 256k 大模型）暴露出模型绑定的结构性问题：

- **摘要任务与对话任务的需求不同质**：摘要是结构化抽取（Goal/Progress/Next
  Steps 骨架），不需要对话模型的完整能力；外部 harness 常见做法是摘要走更快、
  更便宜的模型（本地小模型 / 廉价云端模型），对话继续用主力模型。
- **模型选择权被 harness 收走**：用户能选「用什么模型对话」，却不能选「用
  什么模型压缩」——这个能力应该还给用户。
- 小窗口场景下摘要请求本身也受窗口约束，专用小模型往往与窗口预算更匹配。

c2810 已让摘要请求成为**独立请求且带独立参数**（`max_output_tokens` 接线），
模型独立是其自然延伸；本变更把「模型」也变成摘要请求自己的参数。

## What Changes

（方向性草案，具体取舍 propose/design 裁决）

- **配置面**：compaction 专用模型配置（如 `compaction.model` 引用已配置的
  provider/model），**缺省 = 继承当前会话模型**（现行为零变化，开箱即用）。
- **解析语义**：摘要请求用配置模型构建独立 `XyModel` 实例；配置缺失、指向的
  模型不可用或构建失败时 MUST 回退当前模型（显式降级可观测，不静默换模型）。
- **任务级模型选择的可扩展形态**：落点不是「compact 特判」，而是「按用途解析
  模型」的 seam（任务角色 → 模型引用的解析入口）；compact 摘要是第一个消费者，
  后续同类旁路任务（branch summary、未来 title/标签生成等非对话任务）可复用
  同一解析入口。不预建未落地任务的抽象。
- **观测归因**：摘要请求的 provider-trace / accounting 需能区分实际使用的
  摘要模型与回退形态（复用 `langfuse`/otel 会话归因，扩展请求侧模型标注）。
- **交互面**（propose 裁决）：配置文件为主；是否补 slash 查看当前摘要模型、
  attach/remote 客户端可见性（ModelSelect 类事件 vs 仅 obs）待定。

## Capabilities（预估，正式化时核对）

- `runtime-config`：compaction 配置扩展（model 引用字段，缺省语义）。
- `runtime-model-registry`：按用途解析/构建模型的注册入口（任务角色 seam）。
- `domain-compaction`：c7（LLM 摘要）条款修订——摘要调用可指定模型与回退语义。
- `agent-runtime` / `agent-session`：摘要路径的模型构建与传递。
- `infra-otel` / `package-ai-bridge-accounting`：请求侧模型归因标注。

## Impact

- 行为合约变更（c7 等）→ 完整 SDD pipeline（propose 起）。
- **软相关 c2810**：无硬依赖，但会动同一条摘要调用链
  （`generate_summary` / `compact_session` 签名邻域），建议在其归档后再
  propose，避免并行冲突。
- 兼容红线：未配置时行为与现状完全一致（继承当前模型）；会话 JSONL 与
  compaction 条目形状不变。
- Open Questions（propose 裁决）：model 引用 schema（引用 registry 既有配置
  vs inline provider 配置）；摘要模型的 thinking level / 参数取值来源；回退
  时的用户可见性（obs_diag / 滚动提示 vs 仅 obs）；跨端同源约束板的对应条款。
