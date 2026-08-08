# c1980 预调研：models.dev 原料 ↔ xylitol 字段对照

> 日期：2026-08-08。草案 change：`c1980-add-optional-models-dev-catalog`。
> **不是**产品 MUST；正式 `propose` 前可再刷新。

## 拉取情况

| 路径 | 结果 |
|---|---|
| 本机直连 `https://models.dev/api.json` | **失败**（TCP 连不上 :443） |
| 本机常见本地代理端口（7890/7897/10809/8080/6152/8888 HTTP；7891/1080/10808 SOCKS） | **无响应** |
| Cursor WebFetch → `api.json` | 拉到约 **1MB 截断**，无法整文件 `json.load` |
| GitHub `anomalyco/models.dev` `dev` 分支 | **成功**：`packages/core/src/schema.ts`、`providers/*/provider.toml`、`providers/*/models/*.toml` |
| `raw.githubusercontent.com/.../models.json` | 200 但内容为 OpenRouter 形 `{data:[...]}`，**不宜**当作 models.dev `models.json` SSOT（与 README 描述的 provider-agnostic catalog 不符；待有 catalog.proxy 后重拉官方端点核对） |

结论：草案里 **catalog 专用代理** 不是空想——本环境已复现「LLM 网关可达、catalog 源不可达」。正式实现前用用户提供的 `catalog.proxy` 再拉全量 `api.json` / `models.json` / `catalog.json` 钉死键空间。

本目录保留：schema 快照、openai/anthropic/deepseek 的 `provider.toml` + 若干 model TOML 样例、字段统计 JSON。

## 上游形状（源仓 TOML → 发布 JSON）

权威类型见 `models-dev-schema.ts`（Zod）。与 xylitol 最相关的是 **`ReasoningOption`**：

| `type` | 含义 | 例 |
|---|---|---|
| `toggle` | 仅开关 | DeepSeek Flash 另有 effort |
| `effort` | 离散档名列表 | `values = ["none","minimal","low","medium","high","xhigh","max","default"]` 子集 |
| `budget_tokens` | Anthropic 式预算区间 | `min` / `max` |

Model TOML 还可含：`name`、`base_model`、`cost.*`、`interleaved.field`（如 `reasoning_content`）、`structured_output` 等。

Provider TOML：`name`、`env[]`、`npm`（AI SDK 包名）、可选 `api`（兼容端 base URL）、`doc`。

发布端点（README）：

- `api.json` — 按 provider 聚合（含 `models` 子表、定价、limit、能力）
- `models.json` — provider-agnostic 元数据
- `catalog.json` — 合并包

## xylitol ModelEntry 对照（映射草案）

| xylitol 字段 | models.dev 候选 | 自动？ | 备注 |
|---|---|---|---|
| `provider` (`XyModelKind`) | provider id / `npm` | **否**（映射表） | `@ai-sdk/openai-compatible` ≠ xylitol `openai`/`anthropic` 枚举；须小表 |
| `model`（wire id） | model 键 / `base_model` | 建议 | 别名仍由用户 YAML 定 |
| `base_url` | provider `api` | MAY | 仅 compat 端；官方 Anthropic/OpenAI 常无此字段 |
| `api` | **无直接字段** | **否** | `npm`/`api` 属 AI SDK 世界；须显式映射到 `openai-responses` / `openai-completions` / `anthropic-messages` |
| `compat` | **无** | **否** | DeepSeek 注释写在 TOML 里，不是结构化 `compat=deepseek` |
| `api_key` / secrets | `env[]` 名提示 | 仅提示 | 禁止自动填密钥 |
| `thinking` | `reasoning` 布尔 **或** 非空 `reasoning_options` | MAY | 有 options → true；仅 false 旗标 → false |
| `thinking_levels` | `reasoning_options` 中 `effort.values`（+ 约定 `off`） | **建议原料** | **禁止**静默展开 STANDARD；toggle-only → 建议 `[off]` 或 `[off, on]` 需产品钉 |
| `thinking_level_map` | effort→wire / budget | 部分 | Anthropic `budget_tokens` 无法直接变离散档名；map 仍常要手写 |
| `context_window` | `limit.context`（发布 JSON） | MAY | TOML 样例里未必总有；以 api.json 为准 |
| `tokenizer` | 无稳定字段 | 否 | 仍手写 / 后置 |
| cost meta | `cost.*` | MAY | 进 registry 展示，非开闭关键 |

### DeepSeek 样例（已落盘）

`provider-deepseek.toml` + `sample-deepseek-deepseek-v4-flash.toml`：

- `npm = @ai-sdk/openai-compatible`，`api = https://api.deepseek.com`
- `reasoning_options`: `toggle` + `effort` values `low|high|max`
- 注释已说明 Completions / Anthropic-compat 字段差异（与 c1970 research 一致）

→ xylitol 建议原料示例（**非**自动注册）：

```yaml
thinking: true
thinking_levels: [off, low, high, max]   # 末项=默认 max；off 为产品关档
compat: deepseek
api: openai-completions   # 或 responses / anthropic-messages：须映射表+用户确认
```

### Anthropic 样例

`budget_tokens` + `min`：没有离散档名列表 → catalog **不能**独自生成 `thinking_levels`；最多 `thinking: true` + suggestion「请自声明档或 map 预算」。

## 与草案红线的对齐

1. 默认 OFF；缺 catalog 不阻塞启动 — 本环境已证明缺网常态。
2. suggestion-only；禁止未知条目自动进 registry。
3. `api`×`compat` 显式映射表；`npm` 字符串禁止直接当 AdapterKind。
4. **c1970**：catalog 不得填 STANDARD 五档；`reasoning_options.effort.values` 才是档位原料候选。
5. override / curated pack 应用与上游**同构 JSON**（发布形状），源仓 TOML 仅作理解 schema 的证据，不是运行时输入格式（除非另开 pack 工具链）。

## 正式 propose 前仍欠

1. 用户侧 `catalog.proxy` 拉齐 **完整** `api.json` / `models.json` / `catalog.json`，重做键频率表（本目录 `api-json-partial-field-stats.json` 可能缺失，因截断未解出）。
2. 钉死 override deep-merge 键空间（provider id → model id）。
3. `effort.values` → `thinking_levels` 是否自动加前导 `off`。
4. suggestion 出口（CLI MVP vs TUI）。
5. curated pack 维护面。

## 本目录文件

| 文件 | 内容 |
|---|---|
| `models-dev-schema.ts` | 上游 Zod schema 快照 |
| `models-dev-README.md` | 上游 README 快照 |
| `provider-*.toml` / `sample-*-*.toml` | openai / anthropic / deepseek 样例 |
| `models-json-field-stats.json` | 误拉到的 OpenRouter 形 models.json 统计（仅作警示） |
| `provider-samples-index.json` | 样例索引 |
| `field-mapping.md` | 一页对照表（同本 README 表，便于单开） |
