# c1930 landing（临时）· resume/import 前缀一致

> **非 SSOT**：读码备忘；正式约束进 `design.md` / live specs。
> **本波不做**状态栏。主目标：**用户 resume / import session 后，最终打到 OpenAI Responses API 的 `input`（及稳定前缀相关字段）尽可能与「未退出续跑」一致，避免 Prompt Cache / KV 无故失效。**
> 实验：同包 `xylitol-ai-bridge` + `configs/testing/live-provider.local.yaml`（默认模型）+ Langfuse（`ProviderRequestTrace` 已挂 `langfuse.observation.input` / `usage_details.cache_read`）。

## 1. 用户路径（要对齐的三种）

| 路径 | 今日代码事实 | 对 API 的含义 |
|---|---|---|
| **同进程续跑** | ReAct 内存 `history` → `project_for_llm` → Assembler | 基线「理想前缀」 |
| **resume**（再开进程 / `--session`） | `load_leaf_branch` → `build_context_entries` → `as_agent_message` → 同上 | **必须**与同进程在「冻结旋钮」下产出相同 `input` 前缀 |
| **import JSONL** | `parse_session_jsonl` → 新 session append 全量 → 之后同 resume | 盘面 round-trip 后仍须同前缀（header id 可复用；内容序不得漂） |

公共管线：

```text
SessionEntry[] (leaf + compaction cut)
  → as_agent_message*
  → project_for_llm
  → ResponsesAssembler.assemble(model, msgs, tools, stream, options)
  → HTTP body.input / tools / reasoning / include / store
       └─ ProviderRequestTrace.capture_request_input → Langfuse observation.input
```

## 2. 「前缀」到底含什么（字段级）

Responses 请求里，**跨轮 cache 敏感的稳定前缀**通常是：

| # | 组件 | 来源 | resume/import 风险 |
|---|---|---|---|
| P0 | `input[0]` system/developer 正文 | `AiBridgeGenerateOptions.system_prompt` ← `build_system_prompt` | **高**：含 `Current date:`（默认 `Utc::now()`）+ `cwd` + skills/tools 文案；隔日 resume / 换目录即整段失效（→ c1905） |
| P1 | `tools[]`（名/描述/parameters/strict） | 当前 `ToolSet` freeze 表 | **高**：MCP settle 增工具、热重排（c1900 已闸首轮；resume 后须仍冻） |
| P2 | `input[1..k]` 历史项序与内容 | JSONL → 投影 → convert | **本波主责**：见 §3–4 |
| P3 | 同轮内 reasoning / message / function_call 相对序 | Assembler / c1925 | 乱序=错环+前缀分叉 |
| P4 | `reasoning` / `include` / `store` | thinking_level + WirePolicy | 开关不一致 → body 形变（未必动 input 文本，但影响服务端行为） |
| P5 | `model` / base URL | live-provider / registry | 实验须固定 |

**幂等比较建议**：对 assemble 结果做规范化 JSON（排序 object key 可选；**数组序不可排**）后比：

1. `input` 全文（或 `input[0..n-1]` 去掉本轮新 user）
2. `tools`
3. `reasoning` + `include` + `store`

在线判据：同前缀续跑时 `usage.prompt_cache_read` / Langfuse `cache_read` **resume ≥ warm 同轮**（对齐 c1925 lab 闸）。

## 3. 历史项转换矩阵（P2 细表）

| JSONL `type` / message.role | → AgentMessage | → AiBridge / Responses item | 无损? | 稳定形状 / 字段 |
|---|---|---|---|---|
| `message` + user/assistant/toolResult/… | `Llm`（serde） | user→`input_text`；assistant text→`output_text`；Thinking+合法 `thinkingSignature`→**原样** `type=reasoning`；ToolCall→`function_call`；toolResult→`function_call_output` | Llm **应无损**；thinking 展示文 **不进** output_text | camelCase wire；signature 字符串原样 |
| `message` + `bashExecution` | Env bash | 伪 user：``$ {cmd}\n{out}`` | 有损 | 文案钉死；`excludeFromContext`→整段省略 |
| `compaction` | Env summary | 伪 user：`[Context summary: {summary}]` | 有损 | 文案钉死；位置由 `build_context_entries` 定 |
| `branchSummary` | Env | 同上 summary 形 | 有损 | 同上 |
| `customMessage` display | Env custom | 伪 user：**仅 content**（丢 `customType`） | 有损 | 本波不产品化栏；承认形状 |
| Header / modelChange / thinkingLevelChange / custom / label / sessionInfo | `None` | 不进 view | — | — |
| abort/error assistant | Llm 在盘 | `project_for_llm` **跳过** | 有意省略 | 跳过集合须稳定 |

**顺序不变量**

1. leaf 上 `as_agent_message` 成功序 = 投影序 = `input` 历史序（插入 system 前置之后）。
2. compaction cut：`Compaction` 摘要 + `firstKept` 起保留段（`build_context_entries`）；不得把已摘要轮的 reasoning 造回来（c27）。
3. 同轮：`reasoning` → assistant message → `function_call`。

## 4. resume / import 专属漂移清单（要比实验钉死）

| ID | 漂移源 | 是否本波 | 现象 |
|---|---|---|---|
| D1 | system `Current date:` 跨日 | 声明 only（c1905） | P0 全崩 |
| D2 | cwd / skills / append system 与开会话时不同 | 声明；实验固定 opts | P0 崩 |
| D3 | tools 表 resume 后变长/重排 | 依赖 c1900 冻表；实验固定 tools | P1 崩 |
| D4 | JSONL 再序列化字段序 / 丢 signature | **本波** | P2 崩 / thinking 回放失败 |
| D5 | 非法 signature omit 行为不一致 | **本波**（c1925 已有 diag） | 偶发缺 reasoning 项 |
| D6 | import 后 leaf/parent 重建错序 | **本波**护栏 | P2 序漂 |
| D7 | compaction cut 与同进程不一致 | **本波**护栏 | 摘要位置漂 |
| D8 | 折叠文案被「优化」改字符串 | **禁止无 change** | 伪 user 字节漂 |

## 5. 实验方案（同库 · 默认模型 · Langfuse）

**实现**：`packages/xylitol-ai-bridge/examples/lab_session_prefix_idempotency.rs`（不进 qa）

```bash
cargo run -p xylitol-ai-bridge --example lab_session_prefix_idempotency
XYLITOL_LAB_OFFLINE=1 cargo run -p xylitol-ai-bridge --example lab_session_prefix_idempotency
```

**配置**：`configs/testing/live-provider.local.yaml`
**证据目录**：`/tmp/xylitol-lab-prefix-*`（`summary.json`、`live_request_arm_{a,b}.json`、`live_history.jsonl`）
**Langfuse**：example **不** OTEL 导出；dump 的 `live_request_arm_*.json` 与全量 xylitol 的 `langfuse.observation.input` **同形**（`input`/`tools`/`model`/`store`…）。对照：本机 `LANGFUSE_*`（`~/.config/xylitol/secret.env`）可查 generation；lab 闸仍以 dump 前缀相等 + `cache_read` 为准。

### 首跑 / 复跑（2026-08-06 · Ornith / llama.cpp）

| 项 | 首跑 `/tmp/...7147` | 复跑 `/tmp/...2038` |
|---|---|---|
| offline / live JSONL 哈希 | 相等 | 相等 |
| arm A/B `input[0..-1]` | **相等** | **相等** |
| warm3 → A → B `cache_read` | 519 → 551 → **553** | 512 → 550 → **552** |
| Langfuse API | — | 可达；抽样 `llm.request` 的 `observation.input` 键集与 dump 同形（`langfuse_compare.json`） |
| 固定 system | `Current date: 2026-08-06` + 固定 cwd（防 D1） | 同左 |

### 臂

| 臂 | 步骤 | 通过标准 |
|---|---|---|
| **离线** | fixture → assemble 两次；serde；JSONL 行 | 哈希相等 |
| **A 内存续跑** | 热身 → 续跑 | 记 cache_read + dump body |
| **B resume** | 热身 → JSONL → 新 adapter → 续跑 | 前缀哈希=热身后；cache 不低于 warm3 / 接近 A |
| **C import 形** | JSONL parse → assemble | 与热身后哈希相等 |

## 6. 与相邻 change

| Change | 关系 |
|---|---|
| c1890 | Assembler 唯一入口 |
| c1925 | reasoning 全量回放 + lab 基线 |
| c1900 | tools 冻表 |
| c1905 | date 日界（P0） |
| c1895 | 状态栏 — **不做** |

## 7. specs / apply 意向

- 合约：resume/import 与同进程在固定旋钮下 `input` 前缀幂等
- 单测：离线哈希；lab：在线 cache + Langfuse
- 不强制新 BDD step
