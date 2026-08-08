# Design: c1940 三协议 × named compat

> Change id：`c1940-update-multi-api-named-compat`（原 `…-remove-openai-completions` 已改名）。

## Pi 学到什么

| Pi | xylitol 采纳 |
|---|---|
| `api` 在 **model** 上选协议族 | 已有 `ModelEntry.api` |
| `compat` quirk bag（Completions 更厚） | 命名轮廓 → `WirePolicy`，**不**抄 URL auto-detect |
| auth 与 catalog 分离 | `api_key` 可写在 model，或 env；secret 插值 |
| DeepSeek 官方在 pi 用 Completions | **本波按产品选择**：官方 flash → Responses；Zen free → Completions |
| llama.cpp 在 pi 用 Completions | **不改**用户现网 Responses |

## 字段模型

```text
ModelEntry:
  provider: openai | anthropic | fake
  model: <upstream id>
  base_url?: URL
  api?: openai-responses | openai-completions | anthropic-messages   # L1 协议族
  compat?: generic | deepseek     # L2 方言轮廓 → WirePolicy + body 调整
  api_key?: string                # 插值后明文；省略 = kind env
  thinking / thinking_level_map / context_window / tokenizer / …
```

### 代码组织（bridge）

```text
provider/
  native/     # L1 第一语言：Responses / Completions / Anthropic Messages
  dialect/    # L2 命名方言：deepseek（…）只做 L1 之上的增量调整
  factory.rs  # api × WirePolicy(compat) → Adapter
```

### `compat: deepseek` 行为

| 族 | 行为 |
|---|---|
| Responses | 不发 `include: [reasoning.encrypted_content]`；`store:false`；不发 `previous_response_id` / `prompt_cache_key` |
| Completions | `thinking: { type: enabled\|disabled }` + 可选 `reasoning_effort` |
| Anthropic Messages | `thinking: { type: enabled }`；**不发** `budget_tokens`（上游忽略） |

### 目标配置（用户本波）

```yaml
deepseek-v4-flash-free-zen:     # Completions / Zen（别名后缀 *-zen）
  api: openai-completions
  compat: deepseek
  model: deepseek-v4-flash-free
  base_url: https://opencode.ai/zen/v1
  api_key: "{{ secret.OPENCODE_ZEN_API_KEY }}"

deepseek-v4-flash:              # Responses / 官方（主别名，无通道后缀）
  api: openai-responses
  compat: deepseek
  model: deepseek-v4-flash
  base_url: https://api.deepseek.com
  api_key: "{{ secret.DEEPSEEK_API_KEY }}"

deepseek-v4-flash-anthropic:    # Anthropic Messages / 官方兼容端（*-anthropic）
  provider: anthropic
  model: deepseek-v4-flash
  api: anthropic-messages
  compat: deepseek
  base_url: https://api.deepseek.com/anthropic
  api_key: "{{ secret.DEEPSEEK_API_KEY }}"
```

命名约定：YAML 键 = registry id = 显示名；通道用**后缀**（`*-zen` / `*-anthropic`），不用 `zen-*` 前缀。上游 wire `model:` 可与别名不同、也可多别名共用。

MCP 公开工具名统一为 `mcp__{server_id}__{tool_name}`（`MCP_PUBLIC_DELIMITER="__"` 单点；Claude/Codex 风格；仅 `[a-zA-Z0-9_-]`；段内可含 `-`/`_`，**不** sanitize 掉 hyphen）。**禁止** `mcp:server:tool` 冒号与点号分隔。execute **不得**反解析公开名，adapter 保存 `server_id`/`tool_name`。对照：`docs/research/mcp-tool-public-naming-hyphen-2026.md`。

## 装配

```text
resolve(api, compat) → AdapterKind + WirePolicy
build_adapter_with_wire_policy(...)
```

infra 注入：不再永远 `WirePolicy::default()`，而由 `compat` 解析。

## 不做

- 自动下载 models.dev 全表（后置；本波 YAML 显式条目足够）
- Zen 全模型表 / Gemini
