# Design: c1905 system prompt 稳定 / 可变切分

> 一手证据：[`research/stable-volatile-split-2026.md`](./research/stable-volatile-split-2026.md)；主题底稿 [`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §1。
> 书语不进 live specs。

## 目标

在 `c1890` 已立的 `ContextPolicy` + Assembler 缝上，把 system 组装做成**可标注、可测、可消融**的片段布局：知道哪些字节属 stable / session_env / volatile，并把日历日策略接到 `date_placement`（今日仅占位）。

**不**追求「零动态 system」；**不**实现状态栏 / tool_search / 布局观测本体。

## 已钉决策（Open Questions → design）

| ID | 决策 |
|---|---|
| D1 日界 | **会话钉死日历日**进 system（`session_env`）；**活时刻**走 volatile 通道（`c1895` clock / meta）。**否决**默认「日界改写 system」（伤 Prompt Cache 前缀）。 |
| D2 迁出 | **不**强制迁出 system date / cwd；MAY 后用 `Omit` 演进到无 date。 |
| D3 skills | 元数据标 **stable（会话）**；正文仍走 `$skill` → user 投影。本波 **不**为对齐书语强行挪 StatusBar；顺序 MAY 后置实验。 |
| D4 枚举 | `DatePlacement` 至少：`SystemAsToday`（现状/消融）、`SystemPinnedAtSession`（推荐默认）、`Omit`。 |
| D5 instructions | **保持不写**顶栏 `instructions`；SSOT = prepend system/developer item。 |

## 片段标签（文档 + 单测心智；非第二套字符串 API）

| 标签 | 内容 | 规则 |
|---|---|---|
| `stable` | 默认/SYSTEM 正文、冻表后 tool snippets、context_files、Guidelines、runtime_policy（模式未变）、skills **元数据** | 会话内只增不改心智；热 `apply_*` 允许改字节但须可测 |
| `session_env` | cwd；可选钉死的 `Current date` | 允许进稳定前缀；**cwd ≠ date**（分轨） |
| `volatile` | 秒级时刻、工具计数、git dirty 等 | **禁止**进 system 前缀；→ `c1895` / 工具结果 |

组装顺序（默认路径，与今日对齐并文档化）：

```text
stable body (SYSTEM|custom|default)
→ append / project_context / APPEND_SYSTEM
→ skills 元数据（stable）
→ Guidelines → runtime_policy
→ session_env（date per policy + cwd）
```

## `date_placement` 算法

```text
match date_placement:
  SystemAsToday:
    每次 build 用「今天」日历日（可测：opts.date 注入优先）——字节 ≡ 现状
  SystemPinnedAtSession:
    会话首次组装（或显式 pin）记下 YYYY-MM-DD；后续 rebuild **不得**因跨日改写
    活时间权威不在 system（栏开则 c1895 clock；栏关则接受日历滞后）
  Omit:
    system 不写 Current date 行；cwd 仍可写
```

- **默认板**（`defaults.rs`）：本 change 将 `DATE_PLACEMENT_DEFAULT` 改为 **`SystemPinnedAtSession`**（产品钉板）；`SystemAsToday` 留作消融/对照。
- 钉死点：capabilities / session 生命周期内一次；resume 同 session 须恢复同一 pin（与 `c1930` 前缀幂等对齐：固定 date 测法可复用）。
- **禁止**秒级时间戳进 system。

## 落点（层）

| 组件 | 层 | 说明 |
|---|---|---|
| 片段标签文档 + `build_system_prompt` 分支 | `agent/prompt` | 组装真源；读 `DatePlacement` / 注入的 pin |
| `DatePlacement` 扩容 + defaults | `agent/context_policy` | code-first；无 YAML/env |
| session/capabilities 持有 pin | `agent/capabilities` | 首次 rebuild 钉死；resume 还原 |
| Assembler | `package-ai-bridge` | **不**改布局；继续无 `instructions` |
| StatusBar / clock | — | **out of scope**（`c1895`） |

禁止：`infra` 另拼 system；ReAct 散落改写 date 行。

## Specs landing 意向（绑定分支后）

- **`agent-prompt`**：增补/修订 — 片段标签可观察语义；`date_placement` 行为（钉死 / 今日 / Omit）；同标签集合 → 稳定字节（单测为主，`feature: false` 可辅）。
- **`agent-runtime`**：`ar33` 从「date_placement 占位」升为**真语义**（默认 `SystemPinnedAtSession`；枚举三态）。
- **不**扩 `runtime-config` YAML 键。
- 可执行 GWT：仅当有端到端可观察行为；组装幂等优先 unit。

## 测试缝

| 缝 | 方式 |
|---|---|
| 同 opts + 同 pin → 稳定字节 | unit（延续 `injected_date_is_stable`） |
| `SystemPinnedAtSession` 跨「伪日界」rebuild 不改 date | unit（钉时钟） |
| `Omit` 无 `Current date:` | unit |
| 默认 ≡ 文档化顺序 | unit / 快照 |
| body 无顶栏 `instructions` | 既有 Assembler golden 回归 |
| **不**扩 BDD step 仅为静态存在性 | 与 c1890/pt5 一致 |

## 非目标

- StatusBar / Lane Runtime（`c1895`）
- tool_search（`c1960`）
- Assembler 布局观测（`c1935`）
- context epoch（`c1920`）
- 强制 skills 段重排（可选后置实验）

## Ethics

- risk_level: low
- prohibited_actions: 无标注的隐式动态 system 注入；秒级时间进 system；默认日界改写 system 前缀
- required_evidence: 片段标签文档 + `date_placement` 单测 + pin 跨 rebuild 稳定
- refusal_contract: 不把「零动态 system」写成 MUST
- escalation_policy: 若默认从 pin 改回每日改写 system，须用户确认
