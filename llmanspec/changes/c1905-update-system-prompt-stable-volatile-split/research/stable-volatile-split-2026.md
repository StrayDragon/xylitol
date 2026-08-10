# System prompt 稳定 / 可变切分（一手深挖 · 2026-08-10）

> **范围**：c1905 propose/FF 前证据；**不**改 live specs / proposal / design / tasks。
> **ethics**：risk_level **low**；不把「零动态 system」写成 MUST。
> **姊妹书**：术语仅对照，书语不进 specs。

---

## 1. 现状组装（证实）

`build_system_prompt`（`src/agent/prompt/system.rs`）顺序为：

1. 正文：`system_prompt`（SYSTEM.md）→ else `custom_prompt` → else 沙箱 minijinja 默认（工具片段）
2. `append_prompt`
3. `<project_context>`（context_files / AGENTS.md 等）
4. `append_system_prompt`（APPEND_SYSTEM.md）
5. `<available_skills>`（过滤 `disable_model_invocation`；intro 要求用 read 加载 SKILL.md）
6. `Guidelines:`
7. `<runtime_policy>`（在 APPEND / Guidelines 之后、date/cwd 之前，对齐 pt10）
8. **总是**追加 `Current date: {date}` + `Current working directory: {cwd}`

`SystemPromptOpts.date`：`None` → `chrono::Utc::now()` 日历日；生产路径（`AgentCapabilities` 构造）未设 `date: Some(...)`，仅单测钉死（`system.rs` tests）。每次 `rebuild_system_prompt` 都会重新取「今天」。

`ContextPolicy.date_placement`（`src/agent/context_policy/{mod,defaults}.rs`）：仅枚举 `SystemAsToday` + 默认常量；**未被** `build_system_prompt` / Assembler 读取。已接线：`tools_mode` → mid-turn tools rewrite 闸（`tools_ops.rs`）。`status_bar_mode` / `date_placement` 仍占位（ar33 / c1890 design）。

合约对照：pt1/pt5/pt10/pt11（`llmanspec/specs/agent-prompt/spec.toon`）；ar33（`agent-runtime/spec.toon`）明确 date_placement 为占位。

---

## 2. 日界 × resume × Prompt Cache（选型建议）

### 机制

- OpenAI Prompt Caching：eligible 请求对**精确前缀**匹配；静态内容靠前、可变靠后（[Prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching)）。GPT-5.6+ 默认可在 latest user/tool 放隐式断点；前缀中途改字节 → 从改点起重写/失配。
- 本仓 research §1（`docs/research/responses-context-layout-and-cache-2026.md`）：日历日进 system 单日可稳；隔日 resume 若改写 system date → 整段前缀失效；秒级时间戳禁止进 system。
- c1930 lab：固定 `Current date: 2026-08-06` 做前缀幂等（同文 §5.3）；产品日界留给 c1905。
- ReAct：system 经 `generate_options.system_prompt` 每轮 prepend，**不**写入 history（`react/mod.rs`）。进程重建 / `rebuild_*` 用 `Utc::now()` → 隔日新日字面量；同进程跨午夜且未 rebuild → 日历日可能过时。

### 三方案对照（proposal Open Questions）

| 方案 | 时间感 | Prompt Cache | 与已钉意向 |
|---|---|---|---|
| **(a)** system date + 日界刷新 | 日历准 | **否决为主默认**：日界改 system = 前缀全失效（OpenAI 精确前缀；research §1） | 与「缓存友好显式标注」冲突 |
| **(b)** date 迁状态栏 | 栏内 clock 权威 | system 前缀可稳 | c1895 Q5 已钉默认 coding profile **含 clock**；system 稳定 env 仍可由 c1905 管（`c1895/proposal.md`） |
| **(c)** system 无 date；跨日首轮追加 meta | 依赖 meta 纪律 | 前缀稳 | 与「cwd/date 默认可留 system」（c1895 out of scope）略张力 |

**推荐（建议，非 MUST）**：默认 **会话钉死日历日进 system（`session_env`）+ 活时刻走状态栏/尾插（当 c1895 开栏）**——即 **(b) 的时间权威 + 可选保留钉死日历日**，**不**采用 (a) 的「每日改写 system」。若产品要减 system 噪声，可演进到 (c)。**否决 (a) 作默认**：日界刷新伤 cache 的代价大于「日历日过时」；过时由栏/meta 纠偏。cwd 与 date **分轨**（cwd 会话内常稳，可留 `session_env`）。**不**要求零动态 system。

c1895 后段 ROI 质疑 always-on 仪表盘：即便默认 `status_bar_mode=Off`，c1905 仍应把「活时间」标为 volatile 通道，避免再塞秒级进 system。

---

## 3. 片段标签映射（意向）

| 标签 | 现有段 | 备注 |
|---|---|---|
| **stable** | SYSTEM.md / 默认正文、tool snippets（冻表后）、context_files、Guidelines、runtime_policy（模式未变） | 身份/策略/项目规则；只增不改心智 |
| **session_env** | cwd；**可选**会话钉死的 `Current date` | 允许进前缀；date≠cwd |
| **volatile** | 秒级 clock、工具计数、git dirty 等 | 禁止进前缀；→ c1895 栏 / 工具结果 |

**skills 元数据**：现状在 mid-system（context 后、Guidelines 前）。`$skill` 把 **SKILL.md 正文**注入 **user 投影**，history 留 `$name`（`skill_expand.rs`；pt8）。目录清单是发现面、会话内相对稳 → 标 **stable（会话作用域）**，与 `$skill` 渐进披露对齐。书 Ch2 倾向「元数据末尾通道」（`ai-agent-book/book/chapter2.md` Skills/状态栏节）——工程上 MAY 把 `<available_skills>` 挪到 system 尾（仍在 date/cwd 策略段附近），**不必**为对齐书语强行进 StatusBar。热 `apply_skills` 仍会改前缀字节——可测、可观测即可。

---

## 4. 双份拷贝：`instructions`（证实仍遵守）

c1890 已钉：**不**写顶栏 `instructions`；SSOT = `system_prompt` → `prepend_system_prompt_item`（thinking on → `developer`，off → `system`）（`archive/...-c1890.../design.md` Q2；`openai_responses.rs`）。`ResponsesAssembler` / assemble 路径 **无** `instructions` 键；body 含 `"store": false`。合规。

---

## 5. 依赖 / 边界 / 过时指针

| Change | 边界 | 并行 |
|---|---|---|
| **c1890**（已归档） | Assembler + Policy 钩子；date 占位 | 硬依赖已满足 |
| **c1895** | StatusBar / clock；**不**强制迁出 cwd/date | 日界权威与栏联调；文件冲突用任务切分 |
| **c1960**（非 c1900） | `tools_mode=Search` / tool_search | 正交；c1905 勿实现 search |
| **c1900**（已归档） | MCP 首条门闸 + Full 冻表 | 仅冻表后 skills/tools 散文更稳 |
| **c1935** | 观测 `date_placement` 等布局属性 | 软并行：c1905 定枚举后 c1935 打点 |
| **c1930** | 前缀幂等 lab（固定 date） | 测法可复用；产品算法属 c1905 |

**过时指针（须在 FF design 改正，本文仅标出）**：

- `c1905/proposal.md` Out of scope：`tool_search（→ c1900）` → 应为 **`c1960`**（c1900 = freeze Full）。
- 同提案可并行句仍可用；research §7 映射表一行「主动工具发现 → c1900」亦过时（同文别处已写 c1960）。

---

## Open Questions 决策表（建议）

| ID | 问题 | 建议决策 | 否决/备注 |
|---|---|---|---|
| D1 | 日界 × date | **会话钉死日历日（session_env）+ 活时刻走栏/meta（volatile）** | 否决 (a) 默认日界改写 system |
| D2 | 是否强制迁出 system date | **否**（MAY 后演进到 (c)） | 拒绝「零动态 system」MUST |
| D3 | skills 元数据标签 | **stable（会话）**；顺序 MAY 靠后 | 正文仍走 `$skill` user 投影 |
| D4 | `date_placement` 枚举扩容 | 至少：`SystemPinnedAtSession` / `SystemAsToday`（现状）/ `Omit`（为 (c) 留位） | 组装层真正消费 Policy |
| D5 | instructions | **保持不写** | 回归测：body 无双份 |

---

## FF 补齐 design/tasks 时建议钉的 4 条

1. **接线**：`build_system_prompt`（或等价）按 `ContextPolicy.date_placement` 分支；默认行为字节级 ≡ 现状 `SystemAsToday`，直到显式改 defaults。
2. **标签 + 单测**：片段标签文档化；同标签集合 → 稳定字节；钉 date 的幂等测（延续 c1930 / `injected_date_is_stable`）。
3. **日界产品句**：默认不因跨日改写已钉 system date；活时间权威指向 c1895 clock（栏关时接受日历日滞后或 Omit）。
4. **边界清单**：改正 tool_search→c1960；明确不实现 StatusBar/search/observability 本体；Assembler 继续无 `instructions`。
5. （可选）skills 段顺序实验：移到 system 尾是否改善 cache 美学——产品行为不变则仅测序。
