---
depends_on:
  - c1890-add-responses-context-policy-assembler
  - c1930-update-session-provider-view-contract
---

# Agent 状态栏子系统（可插拔 / 可验证）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §3（术语对照 §7）
> **书指针**：《深入理解 AI Agent》Ch2「Agent 状态栏：通过元信息增强 Agent 轨迹管理」（姊妹仓 `ai-agent-book/book/chapter2.md`）；书语仅经 research §7 术语表映射，**禁止**写入 live specs。
> **自包含**：本波交付 **Lane Runtime**（Harness 观测 + 注入）与 **Lane Agent 扩展壳**（接口/空实现）；完整 TODO/计划业务 → [`c1896`](../c1896-add-status-bar-agent-lane/proposal.md)。不实现 MCP search / 压缩。持久化/投影标记遵守 `c1930`。
> **工程约定（本波次）**：策略默认 **code-first**：`defaults.rs` 纯常量（改文件调试）；**不**新增 YAML 旋钮；**不**用 env 当未暴露配置面。用户面 YAML 仅既有字段（如 `api`）。真源见 [`c1880`](../archive/2026-08-04-c1880-update-responses-first-api-boundary/proposal.md)。

## Why

状态栏不是银弹：coding agent 下 cwd/date 进 system 常常正确；但工具计数、TODO、git 等**高变读数**若塞进 system 或每轮无策略追加，会在「cache」与「注意力/冗余」之间失控。需要**独立、可测**的子系统，由 ContextPolicy 选择 `off` | `replace` | `append`。

## What Changes

- **双列地基（深挖 Q1，已钉）**：
  - **Lane Runtime**：代码可观测状态 → 结构化键值；**每个 outbound LLM generate 前**按 profile+预算盲目尾插并持久（深挖 Q6′）；本波 **无** refresh 工具（→ [`c1898`](../c1898-add-statusline-refresh-tool/proposal.md)）。
  - **Lane Agent**：可扩展 typed 尾插通道；本波只留扩展接口/空壳；业务 TODO → `c1896`。
- StatusBar **provider 接口**（Runtime）：输入 = 代码可观测状态；输出 = 结构化键值（禁止散文堆砌为默认）。
- Runtime 默认模式：**`append`（深挖 Q2 已钉）**——**不**扫描轨迹中已有 status message；在自动缝上直接追加最新快照（「盲目尾插」）。`replace` / `off` 仍可切。
- **持久化（深挖 Q3 已钉）**：**全部写入 session transcript**（SSOT），不是仅请求时投影。理由：常用 provider 的 KV / Prompt Cache 依赖跨请求前缀字节稳定；仅投影等于每轮换掉末尾条，破坏「只追加」命中。导出 / resume / compact（`c1897`）均可见这些特殊标记消息。
- **Runtime 读数地基（深挖 Q4′ 已钉）**：**不**把字段表钉死为合约。做成
  - `ReadingProvider` 注册表（id / 优先级 / 估 token / 渲染 KV）
  - × **scenario profile**（code-first；coding 默认档对齐书 Ch5 环境感知意向）
  - × **每条 append 硬 token 预算**（超限按优先级丢低优字段）
  - 本波：接口 + 预算 + **薄 coding profile** 可测实现（**默认含 clock**，深挖 Q5）；时间感操作手册 / git 深度 / 更多场景档后置迭代。对齐书实验 2-8「技术可独立开关」与「场景会变 + 省 token」。
- **自动注入缝（深挖 Q6′ 已钉）**：① **每 outbound generate 前**自动尾插；② 本波不做按需工具。后置 `statusline_refresh` = **仅 tool result**、不写权威栏（`c1898`）。
- **标记 / wire（深挖 Q7 已钉；命名意向）**：SSOT 独立 entry kind 统一为 **`AgentStatusBar`**（名以实现为准）；投影包装标签统一为 **`<agent_status_bar>…</agent_status_bar>`**（书实验里的 `<agent_status>` 仅作概念同源，工程不混用短名）。压缩/导出认 kind；不以普通 user 正文标签为唯一 SSOT。与 `c1930` 对齐投影契约。
- **薄 coding 默认附加集（深挖 Q8，已钉）**：
  - **默认开**：`clock`；`tool_calls`（按工具名累计次数——书实验 2-8「工具调用计数器」，也是弱模型防空转/死循环的主读数）。
  - **实现但默认关**（profile 可开）：`cwd`；`git_branch`。
  - **本波不做**：完整 git dirty、TODO、详细错误四层、长操作手册、`statusline_refresh` 等（见分流草案）。
  - 防循环：书证——显式次数（如 `read_file: 3`）能触发「多次失败后换策略/放弃」；电话实验里「3/3 到顶就停」规则足够显然时**只靠读数**即可纠偏。更细的「同参重复 streak / 微型策略」后置，不进本波默认。
- **栏体形状（深挖 Q9 意向：XML 非 JSON）**：投影包装根标签 **`<agent_status_bar>`**；读数用子元素，**不用 JSON 对象当正文**。推荐骨架：

```xml
<agent_status_bar>
  <clock>2026-08-05T14:25:00+08:00</clock>
  <tool_calls>
    <tool name="read" count="3"/>
    <tool name="bash" count="5"/>
  </tool_calls>
  <!-- profile 开启时再出现：
  <cwd>/path</cwd>
  <git_branch>main</git_branch>
  -->
</agent_status_bar>
```

  - `tool/@name` 用属性（工具名可含非法 XML 名字符时仍安全）；`count` 为非负整数。
  - 空 `tool_calls` 可写成 `<tool_calls/>` 或省略子节点（design 钉一种）。
  - **禁止**把整段栏序列化成 `{...}` JSON 塞进 user 正文当默认形。
- 注入经 Assembler / Policy，**不**散落改 `build_system_prompt` 特例逻辑（system 内稳定 env 仍可由 `c1905` 管）。
- 验证：假 provider → 预算截断可测；profile 开/关 `cwd`/`git_branch` 可消融；generate 边界尾插含 `clock`+`tool_calls`；entry kind=`AgentStatusBar`；投影 XML 可解析/快照。
- **禁止**用 LLM 批量扫历史生成权威栏。

## Capabilities（意向）

- `agent-*`（status bar / runtime lane）
- ContextPolicy `status_bar_mode` 真消费
- 产品文：压缩与上下文 / 新 architecture 短节（归档时）

## Impact

- 弱模型/长轨迹可按需打开；默认策略以深挖钉板为准。
- 与 cache 优化解耦：开栏不等于追求命中。
- Agent 列扩展点避免后继挤进 Runtime。

## Out of scope

- 把 cwd/date **强制**迁出 system（本仓场景默认可留；`c1905` 可标 stable）
- tool_search（→ `c1900`）
- 完整 TODO / 即时计划产品形态（→ `c1896`；本波仅扩展壳）
- 压缩时对特殊标记 status message 的保留策略（→ `c1897`；本波只保证可识别标记）
- 按需 `statusline_refresh` 工具（→ `c1898`；仅 tool result，本波不做）
- 子 agent 字节级对齐父栏（后置）

## Parallel / depends

- **硬依赖**：`c1890`（已归档）、`c1930`
- **分流草案**：
  - [`c1896`](../c1896-add-status-bar-agent-lane/proposal.md)（Agent 列；sourced_from 本 change）
  - [`c1897`](../c1897-update-compaction-status-bar-messages/proposal.md)（压缩 × status；sourced_from 本 change）
  - [`c1898`](../c1898-add-statusline-refresh-tool/proposal.md)（按需 refresh 工具；sourced_from 本 change）
- 可与同层无硬依赖冲突的 change 并行（不同文件/模块优先）

## Open Questions

### 已解决

- **Q1 双列范围（2026-08-05）**：选 **双列 + 本波只通 Runtime**；Agent 列留可扩展接口/空壳；TODO 业务形态未定 → 想法写入 `c1896`（标记 sourced_from 本 change）。
- **Q2 Runtime 默认模式（2026-08-05）**：选 **`append` = 盲目尾插**（不查看轨迹中已有 status message，直接追加到末尾）。`replace`/`off` 仍为可切档。压缩时陈旧 status 堆积 → 策略延后调研，写入 `c1897`（最多保留一条 vs 全不保留，未定）。
- **Q3 持久化（2026-08-05）**：选 **全部持久进 transcript**（非仅请求投影）。动机：保住常用 LLM provider 的跨请求 KV / Prompt Cache（append 前缀稳定）；仅投影会每轮替换末尾条、破坏命中。`c1897` 因此更关键。Agent 列（`c1896`）默认同源持久，除非后继另钉。
- **Q4′ Runtime 读数地基（2026-08-05）**：选 **注册表 × scenario profile × 单条 token 预算**；本波薄 coding profile，**不**把具体键表钉成硬合约。书据：Ch2 实验 2-8 可独立开关；Ch5 coding 环境四件套为 profile 意向而非 SSOT；append 持久下省 token 靠单条预算 + 后继 `c1897`。
- **Q5 时钟 / 日界（2026-08-05）**：选 **C — 栏内 clock provider，并进入默认 coding profile**（每轮盲目尾插带时间读数）。system/`c1905` 仍可保留稳定 env 策略，但「当前时刻 / 日历日」以栏为准避免改 system 前缀；单条预算须为 clock 留优先级；与 `c1905` 日界文案对齐时注明「动态时刻走栏」。
- **Q6′ 自动缝 × 按需工具（2026-08-05）**：分类后选 **① 每 outbound generate 前自动尾插 + ② 本波不做 refresh 工具**。后置工具若做：名 ≈ `statusline_refresh`，**仅 tool result、不 append 权威栏** → `c1898`。避免与 `c1897` 双写缠死。
- **Q7 标记 / wire（2026-08-05）**：选 **A — 独立 session entry kind + 投影层包装**。工程命名统一：**`AgentStatusBar`** + **`<agent_status_bar>`**（不用混用 `StatusBar` / `<agent_status>` 短名）。压缩/导出认 kind；跟 `c1930` 联调投影细节。
- **Q8 默认附加集（2026-08-05，已被 Q10 覆写）**：曾钉 `clock`+`tool_calls`；经 ROI 深挖后见 Q10。
- **Q9 栏体形状（2026-08-05）**：选 **XML 子树**（非 JSON）。根 `<agent_status_bar>`；读数用子元素。

### ROI / 姿态复核（深挖 Q10，2026-08-05）

**结论（待用户确认选档）**：对 xylitol **个人 coding agent**，默认 always-on 仪表盘（`clock` / 累计 `tool_calls`）**边际收益低**；append+persist 下每 generate 一条会堆 **陈旧税**。书里状态栏硬证据在「弱模型 + 显然约束的计数/状态跟踪」；coding 日常更吃 **硬闸（max attempts）+ 显式 Todo/事件 + 工具结果反馈**。高时效信息（刚变的 cwd、进行中后台任务）更适合 **事件触发注入或 tool result**，不宜当「过时仍权威」的栏优化。

**更适合进栏（稀疏、代码真源）**：工具达 max-attempt 的约束帧；agent 登记的 follow-up/事件；Todo 完成度摘要（SSOT 在 Todo，栏只投影）；cron/后台完成、accounting 阈值等 **模型无法从轨迹可靠推断** 的外部事实。

**Todo+TUI**：产品 SSOT；栏 / Agent 列只是模型注意力通道——**禁止**栏 auto 摘要当 Todo 真源。

### 待钉（产品姿态）

- 选 **(D)+(C)**：大幅降权 always-on；改为 **事件/Todo 驱动才出现**；c1895 最多薄缝且默认 `off`
- 选 **(A)**：只留薄缝（kind / mode / 钩子），默认 `off`，内容全后置
- 选 **(B)**：仍 ship 最小 always-on（不推荐）
- Todo 产品是否升为优先 change（可扩 `c1896` 或新 id），状态栏跟其后

## Ethics

- risk_level: medium（高信任注入面；且持久后进入导出/resume）
- prohibited_actions: LLM 维护权威栏；把外部不可信全文写入栏；不可审计的隐式投毒通道；无预算的无限膨胀 profile；用普通 user 正文冒充 `AgentStatusBar` kind；默认栏体用 JSON 对象冒充结构化读数；用栏冒充 Todo SSOT
- required_evidence: 姿态选定后：mode 默认可测；事件触发注入可测（若选 C/D）；勿在策略未定前默认 always-on append
- refusal_contract: 不宣称状态栏普遍提升正确率；不宣称计数器 alone 能消灭所有循环；不宣称调栏是高 ROI 默认投入
- escalation_policy: 若默认从 `off` 改为 always-on append，或默认预算显著放大，须用户确认
