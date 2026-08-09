# 字段对照一页纸（xylitol ModelEntry ↔ models.dev）

| xylitol | models.dev 候选 | 自动注册？ |
|---|---|---|
| `provider` | provider id / `npm` | 否（显式映射表） |
| `model` | model id / `base_model` | 建议值 only |
| `base_url` | provider `api` | MAY |
| `api` | （无）← 禁止用 `npm` 冒充 | 否 |
| `compat` | （无；注释里） | 否 |
| `thinking` | `reasoning` / 非空 `reasoning_options` | MAY |
| `thinking_levels` | `reasoning_options` `effort.values` (+ 产品 `off`) | 建议原料；禁 STANDARD |
| `thinking_level_map` | effort/budget 提示 | 部分；Anthropic 预算常手写 |
| `context_window` | `limit.context`（api.json） | MAY |
| `tokenizer` | — | 否 |
| cost 展示 | `cost.*` | MAY |

详见同目录 `README.md`。
