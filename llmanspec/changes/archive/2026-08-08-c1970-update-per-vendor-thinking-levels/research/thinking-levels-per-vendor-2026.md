# 调研：按厂商 thinking 旋钮对照（c1970 门禁）

> 日期：2026-08-08。一手来源：DeepSeek 官方 Thinking Mode 指南；对照 xylitol `packages/xylitol-ai-bridge` 现状与会话 resume 路径。
> 用途：支撑 `c1970-update-per-vendor-thinking-levels` design / specs；**不是**产品 MUST 正文。

## 结论摘要

1. **没有统一尺子**：OpenAI effort 枚举、Anthropic `budget_tokens`、DeepSeek 的 toggle+effort（且三协议字段不同）不能共用一把「STANDARD 五档」产品梯子。
2. **DeepSeek 近端用户档宜短**：官方 effort 侧常见 `low` / `high` / `max`（另有映射表里的 `xhigh`）；Responses 用 `reasoning.effort` 的 `none` 关 thinking。产品面用配置声明的短列表（如 `off,high,max`）比塞满 STANDARD 更诚实。
3. **xylitol 今日问题**：`ThinkingLevel::STANDARD` 在 `thinking: true` 且未声明列表时静默展开；bridge 已按 **字符串** resolve，但协议层仍用封闭枚举校验配置。
4. **resume / 重放两层**：
   - **档位选择**：`ThinkingLevelChangeEntry.thinking_level: String` 已是无损落盘；`build_session_context` 按分支末次条目还原字符串。危险点是 **load 后 clamp 写回** 或换模静默改档而不留痕。
   - **推理内容重放**：Responses `thinkingSignature` 全量回放（既有 pab25）与档位选择正交；本 change 不改回放策略，但须保证 resume 后 **同一档位字符串** 继续进入 `AiBridgeGenerateOptions.thinking_level`。
5. **catalog（models.dev）**：档位原料可后置给 `c1980`；本 change 以 **配置显式 `thinking_levels`** 为正途，不维护「全球超集枚举」。

## 厂商旋钮表

| 表面 | 关 thinking | 强度 / 预算 | 备注 |
|---|---|---|---|
| OpenAI Chat Completions | 省略 `reasoning_effort` | `reasoning_effort` 字符串（常见 minimal/low/medium/high…） | xylitol generic Completions 路径 |
| OpenAI Responses | 省略 `reasoning` | `reasoning.effort` + `summary`（xylitol 默认 `auto`） | deepseek compat 拒 `include: reasoning.encrypted_content` |
| Anthropic Messages | 省略 `thinking` | `thinking: { type: enabled, budget_tokens }` | 官方是 token 预算，不是同名档枚举 |
| DeepSeek Completions | `thinking.type=disabled` 或等效 | `reasoning_effort`：`low`/`high`/`max`（文档映射另含 `xhigh`） | SDK 常把 `thinking` 放 `extra_body`；默认 thinking on、effort high |
| DeepSeek Responses | `reasoning.effort=none` | `reasoning.effort`：`none`/`low`/`high`/`max` | 与 Completions 字段形状不同 |
| DeepSeek Anthropic-compat | 文档：`output_config.effort`；xylitol 现状：`thinking.type` 且 **不发** `budget_tokens` | 服务端忽略 budget | 见 `dialect/deepseek.rs` |

### DeepSeek 请求 effort → 实际映射（官方表，2026-08）

| Requested | v4-flash 实际 | v4-pro 实际（文档称 early Aug 2026 可能更新） |
|---|---|---|
| low | low | high |
| high | high | high |
| xhigh | high | max |
| max | max | max |

含义：即使用户选 `low`，Pro 可能仍跑 high——**产品若展示虚假精细档会误导**。

### Tool 多轮

DeepSeek：带 `tools` 时后续请求 **必须**回传 `reasoning_content`，否则 400。这与 xylitol 的 signature / content 回放相关，属 bridge 消息投影，不在「档位列表」本 change 主切片，但 design 须标明 **勿在改档位时破坏 CoT 回传**。

## xylitol 代码事实

| 点 | 位置 / 行为 |
|---|---|
| 封闭枚举 + STANDARD 默认 | `src/protocol/model/thinking.rs`：`resolve_configured_levels` 无列表 → STANDARD |
| 配置拒未知枚举名 | `validate` / parse：非 off…max 加载失败 |
| Bridge 已字符串化 | `AiBridgeGenerateOptions.thinking_level: String`；map 键为字符串 |
| 会话档位条目 | `ThinkingLevelChangeEntry { thinking_level: String }` |
| 上下文还原 | `SessionManager::build_session_context`：扫分支，末次 ThinkingLevelChange 覆盖；缺省字面量 `"medium"`（无条目时） |
| 换模默认 | m10：声明列表末项；Settings.default 不得覆盖换模末项策略 |

## 对本 change 的设计含义

1. **未配置 `thinking_levels` 且 thinking 可开**：支持集 = 仅 `off`（不可调）。禁止 STANDARD 静默展开。
2. **配置列表 = 用户面 SSOT**：任意非空档名字符串（至少保留字面 `off` 语义）；**不**要求属于历史枚举超集。未知「枚举成员」不再是加载失败理由；非法改为空串 / 重复等结构错误。
3. **无全序时的换模默认**：采用 **配置列表末项**（可调时）；仅 `off` → `off`。Settings.default 仅会话首次装配且须 ∈ 支持集（沿用 rc16 精神）。
4. **resume 无损**：
   - 还原末次 `thinking_level` 字符串，**禁止**因「不在新支持集」而改写 JSONL 或自动追加 clamp 条目。
   - 允许短暂 **sticky out-of-set**（配置漂移）：generate 仍用该字符串 + map；用户显式 `set`/`cycle` 才落到集内并落盘新条目。
   - 推理 signature 全量回放策略不变。
5. **`thinking_level_map`**：键 ⊆ 声明列表（或任意已用档名）；值仍为 wire 字符串或 `null` 省略。Anthropic 数字预算可继续用 map 值。
6. **与 c1980**：catalog 可日后建议 `thinking_levels` 原料；本 change 不依赖远程 catalog。

## 一手链接

- DeepSeek Thinking Mode：https://api-docs.deepseek.com/guides/thinking_mode
- DeepSeek Anthropic base：`https://api.deepseek.com/anthropic`
