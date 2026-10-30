# 设计：输出预算接线

## 字段与映射

- `XyModelEntryConfig.max_output_tokens: Option<u32>`；`to_model_meta` 把声明值转成 `XyModelMeta.max_tokens: u64`，未声明为 0。
- `Some(0)` 视为非法并返回配置错误，与 `input_modalities` 的校验路径一致（注册期跳过并告警）。

## 运行时接线

- `prepare_turn_binding` 从当前 `XyModelMeta` 取 `max_tokens`：非零 → `Some(u32)`，0 → `None`。
- 纯映射抽成函数以便单测，不引入额外状态。

## 适配器

- Responses：`options.max_output_tokens` → `max_output_tokens`；未提供则省略。
- Completions：`options.max_output_tokens` → `max_completion_tokens`；未提供则省略。
- Anthropic：`options.max_output_tokens.unwrap_or(8192)`；8192 作为安全缺省保留。

## 为什么只认配置

用户决策：配置优先级高于 models.dev；仓库不抓取外部目录，避免网络与缓存面。

## 风险

- 用户配置过小会截断回复；属显式配置后果。
- Anthropic 的安全缺省 8192 对长输出模型偏小，但保持既有行为，不扩大本切片。
