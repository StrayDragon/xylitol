# 官方 models.dev JSON × 源仓 TOML 交叉校验

> 经本机 egress 拉取 `https://models.dev/{api,models,catalog}.json`（完整 blob 仅留 `/tmp`，仓库只存 stats + sha256）。
> 详细数字：`api-json-field-stats.json`。源仓侧：`source-toml-field-stats.json` / `api-json-via-source-toml.md`。

## 端点结论

| URL | 结果 | 用途建议 |
|---|---|---|
| `api.json` | ~3.5 MiB，provider→nested models | **MVP merge SSOT** |
| `models.json` | ~249 KiB，flat ~301 条 | 精选子集，非全表 |
| `catalog.json` | ~3.7 MiB，`{models,providers}` | models 亦 ~301；可作 slim 视图 |
| `providers.json` | HTML SPA | **不要当 catalog** |

## 与源仓对齐

| 维度 | TOML | api.json | 判定 |
|---|---:|---:|---|
| providers | 181 | 181 | 一致 |
| `api` URL 有值 | 155 | 155 | 一致 |
| `@ai-sdk/openai-compatible` | 143 | 143 | 一致 |
| model 条数 | 2913 文件 | **6231** nested | 发布 JSON 含扩展/镜像，多于一文件一模 |
| `reasoning_options` 类型 | effort/toggle/budget_tokens | 同左（数组项） | 语义一致；计数因模型膨胀而更大 |
| effort 字面量 | high/low/medium/max/none/xhigh/minimal… | 同左 + 极少 `None`/`default` | 建议档仍：精确 `off` + `values[]` |

## 产品映射（确认）

1. refresh 目标：**`api.json`**（不是 models.json 全表幻想）。
2. `thinking_levels` 建议：仅离散 `effort.values`，前加精确 `off`；`toggle` / `budget_tokens` 不伪造成五档 STANDARD。
3. npm → api×compat：小显式表；未知 npm 只建议元数据，须手写协议字段。
4. suggestion-only：不自动 register。
