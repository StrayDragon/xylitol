# 设计：结构化输出

## 选项形状

- `XyStructuredOutput { name: String, schema: Value, strict: Option<bool>, description: Option<String> }`，`strict` 缺省按 `false` 发送（OpenAI 要求显式布尔）。
- `XyGenerateOptions.structured_output: Option<XyStructuredOutput>`，缺省 `None`。

## 映射

- Responses：`text.format`，含 `name` / `schema` / `strict`；`description` 暂不发送。
- Completions：`response_format.json_schema`，含 `name` / `schema` / `strict`。
- Anthropic：`output_config.format`，仅 `type` 与 `schema`。

## 为什么没有配置面

结构化输出是调用级需求（eval / 工具参数约束），不是模型固有属性；放进 `XyGenerateOptions` 即可由嵌入方设置，避免模型条目字段膨胀。

## 风险

- `name` 为空时 OpenAI 可能拒绝；由调用方保证非空。
- Anthropic beta 字段若未开通会 400；错误已结构化（S7），调用方可判别。
