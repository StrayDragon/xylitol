---
name: "llman-sdd-arch-review"
description: "扫描 codebase 的薄模块（接口几乎等于实现），找出可以加深（藏更多行为到更小接口后）的候选。当用户想做架构审查、寻找模块加深机会、或想改善代码可测性与 AI 可导航性时使用。"
metadata:
  version: "0.0.72"
---

# LLMAN SDD Architecture Review

扫描 codebase 的架构摩擦，找出**可以加深的模块**——把薄模块（接口几乎等于实现）改造成厚模块（小接口后藏大量行为）。目标是可测性与 AI 可导航性。

## Pipeline 位置

辅助工具，不属于主实现 pipeline（explore→propose→apply→verify→archive）。任意阶段可用，常在 explore 阶段触发以发现改进候选。

> 📍 这是独立可选 skill，不替代任何 pipeline 阶段。

## 设计词汇

下面是一组关于模块形状的词，用来说清楚「哪里值得改」。MUST NOT 替换为「component」「service」「API」「boundary」（它们含义更宽、不够精确）：

- **Module（模块）** — 有接口和实现的东西（函数/类/包/跨层切片都算）。
- **Interface（接口）** — 调用者为正确使用所须知道的一切：类型签名，外加不变量、顺序约束、错误模式、性能特征。
- **Depth（厚度）** — 接口背后的行为量。**厚** = 小接口后藏大量行为；**薄** = 接口几乎和实现一样复杂（调用者要懂的 ≈ 写代码要写的）。本 skill 要把薄的变厚。
- **Seam（接缝）** — 不改调用处就能换实现的位置（接口栖身的地方）。llman 里接缝 = `*.feature` 的 GWT 步骤所驱动的公共边界。
- **Leverage（杠杆）** — 调用者从厚度获得的好处：学一点接口就能驱动很多行为。
- **Locality（局部性）** — 维护者从厚度获得的好处：变更/bug/知识/验证集中在一处，改一次到处生效。

## 步骤

### 1. 探索（先定范围，YAGNI）
- 若用户指定了方向（模块/子系统/痛点），直接采信，跳过推断。
- 否则回看 `git log --oneline` 找热点（反复出现的文件/区域）。
- 优先读 live `<capability>.feature`（单轨 SSOT）与 `design.md`（已有 ADR），MUST NOT 另建 `CONTEXT.md`。
- 用 Agent 工具（subagent_type=Explore）走查 codebase，记录摩擦点：
  - 理解一个概念是否要在多个小模块间跳来跳去？
  - 哪里模块**薄**（接口几乎和实现一样复杂，调用者没省事）？
  - 哪里纯函数仅为可测性抽取，但真实 bug 藏在调用方式里（缺局部性）？
  - 哪些部分没测或难以通过当前接口测试？

### 2. 提出候选
对每个候选，给出：
- **Files** — 涉及哪些文件/模块。
- **Problem** — 当前架构为何造成摩擦（用厚度/杠杆/局部性说清楚）。
- **Solution** — 会改变什么的平实描述。
- **Benefits** — 局部性与杠杆的改善，测试如何变好。
- **Recommendation strength** — `Strong` / `Worth exploring` / `Speculative`。

**删除验证**：对任何疑似薄的模块，想象删除它——复杂度是直接消失（它只是个透传，没价值）还是在 N 个调用点重新冒出来（它其实在扛事）？「重新冒出来」才是值得保留/加厚的信号。

**ADR 冲突**：若候选与既有 `design.md` 决策矛盾，仅在摩擦真实到值得重开时才浮现，并在候选中标注（「与 design.md 的 X 决策冲突——但因…值得重开」）。

### 3. 逐问深挖（用户选定候选后）
用户从候选中选一个后，运行 `llman-sdd-explore` 的**逐问深挖分支**（触发词「深挖」）逐个走清决策——约束、依赖、加深后的模块形状、接缝后放什么、哪些测试存活。

- 加深后的模块用到了 capability `.feature` 里没有的概念？→ 仅在 change 已 Branch binding 且当前在绑定分支上时，更新 live `.feature`（Specs landing）；否则 STOP，先走 `llman-sdd-propose` / `change start`，**禁止**在默认分支改 live specs。
- 用户以关键理由拒绝候选？→ 仅当「难逆转 + 无上下文会困惑 + 真实权衡」三者皆满足时，建议记入 `design.md`。

## 输出
候选清单（文本；可选 HTML 报告写 OS temp dir 不落 repo）+ 用户选定后的逐问深挖决策记录（回写 proposal；合约变更须经 Specs landing 才回写 live `<capability>.feature`）。

## Context
- 先查状态再动手：change/spec 状态以 `llman sdd show/list/validate` 输出为准。
- 读 spec 全文前先用 `llman sdd context --task --paths` 定位相关 specs。

## Goal
- 本节命令达成一个可验证结果；结果路径与校验状态随报告输出。

## Constraints
- 遵守正文「硬约束/硬规则」，本节不复读。先判断变更规模选路径（triage）：行为合约变更走完整 SDD，实现层走 quick；不确定选完整 SDD（保守）。
- 改动保持最小；已知校验错误禁止强行继续。

## Workflow
- 每步以 `llman sdd` 命令结果为事实来源；改动工件后必跑 `llman sdd validate`。
- 命令细节见下方生成式命令参考或 `llman sdd <cmd> --help`。

## Decision Policy
- 高影响歧义先澄清再继续；事实自己查证，只有决策问用户。

## Output Contract
- 报告先给人读摘要（结论 / 风险 / 待决策），机器细节随后。

## Ethics Governance
- `ethics.risk_level`：low——仅读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时从其声明。
- `ethics.prohibited_actions`：违反正文「硬约束」的动作；未经用户明确要求的 push / PR / 外部上传。
- `ethics.required_evidence`：结论须有命令输出或文件路径佐证；门禁状态以 `llman sdd validate` 为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零 → 拒绝进入下一阶段；自修复达上限 → 报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约/模板或执行不可逆动作前，暂停并请用户确认。
