# Design — c1165-add-runtime-thinking-level-map

## 已拍板

| 决策 | 选择 |
|---|---|
| 配置字段 | per-model YAML `thinking_level_map`（snake）；值 `string` 或 `null` |
| 语义（对齐 pi） | **键缺省** → adapter 内置默认；**字符串** → 原样发给 provider；**`null`** → 该档不发 thinking/effort 字段 |
| 校验 | 未知键名（非 `ThinkingLevel::as_str`）→ **配置加载失败** |
| 调用缝 | 新增 `XyGenerateOptions { thinking_level, level_map, thinking_budgets }` 传入 `XyModel::generate_stream`；默认 `Off` + 空 map = 今日行为（多数路径不添字段） |
| Anthropic v1 | **budget** 路径：`thinking: { type: "enabled", budget_tokens }`；Off → 省略或 `disabled`；budget 来自 Settings `thinking_budgets` 缺省用内置表 |
| OpenAI Completions | `reasoning_effort`（字符串） |
| OpenAI Responses | `reasoning: { effort }` |
| Map 存放 | `ModelEntry` → `XyModelMeta.thinking_level_map`；**不**放进 `XyModelConfig` |
| Adaptive Anthropic | **本变更不做** |

## 内置默认（键缺省时）

| Adapter | Off | minimal…high（及 xhigh/max） |
|---|---|---|
| OpenAI Completions / Responses | 省略 effort 字段 | effort = level `as_str()`（identity） |
| Anthropic budget | 省略 `thinking` 块 | `budget_tokens`：minimal=1024, low=2048, medium=8192, high=16384, xhigh/max=32768（可被 Settings 覆盖） |

用户 map 覆盖：例如 `high: "max"` 或 `off: null`。

## 数据流

```text
ModelManager.thinking_level()
  + XyModelMeta.thinking_level_map
  + Settings.thinking_budgets
  → react call_with_retry → XyGenerateOptions
  → AdapterXyModel → bridge generate_stream
  → resolve_thinking_for_request → 写入请求 JSON
```

## 风险

- 改 `XyModel` 签名会触所有 impl（Fake/Mock/Scripted/bridge）——用带 Default 的 options 参数，一次性改完
- Anthropic 无 thinking 能力的模型：Off/null 必须安全省略，不得硬塞 enabled
