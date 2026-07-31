---
name: "llman-sdd-arch-review"
description: "扫描 codebase 的薄模块（接口几乎等于实现），找出可以加深（藏更多行为到更小接口后）的候选。当用户想做架构审查、寻找模块加深机会、或想改善代码可测性与 AI 可导航性时使用。"
metadata:
  version: "0.0.66"
  llman_sdd:
    bdd_mode: "on"
    skill_set: "optional"
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
- 优先读 live `spec.toon`（BDD-on，领域语义 SSOT）与 `design.md`（已有 ADR），MUST NOT 另建 `CONTEXT.md`。
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

- 加深后的模块用到了 `spec.toon` 里没有的概念？→ 仅在 change 已 Branch binding 且当前在绑定分支上时，更新 live `spec.toon`（Specs landing）；否则 STOP，先走 `llman-sdd-propose` / `change start`，**禁止**在默认分支改 live specs。
- 用户以关键理由拒绝候选？→ 仅当「难逆转 + 无上下文会困惑 + 真实权衡」三者皆满足时，建议记入 `design.md`。

## 输出
候选清单（文本；可选 HTML 报告写 OS temp dir 不落 repo）+ 用户选定后的逐问深挖决策记录（回写 proposal；合约变更须经 Specs landing 才回写 live `spec.toon`）。

## Context
- 执行前先确认当前 change/spec 状态。
- 优先使用 `llman sdd context --task --paths` 获取相关 specs，而非全量读取或猜测。

## Goal
- 明确本次命令/skill 要达成的可验证结果。

## Constraints
- 变更保持最小化且范围明确。
- 标识符或意图不明确时禁止猜测。
- 在读取 spec 全文前，先使用 `llman sdd context --task --paths` 获取相关 specs。
- 判断变更规模后选择路径：行为合约变更走完整 SDD（Branch binding → Specs landing → `readyToImplement` → apply）；实现变更走快速路径（live specs 仍须绑定分支）。
- 勿混淆 Skill 导航与 Git-native 生命周期；勿在默认分支编辑 live `llmanspec/specs/**`。

## Workflow
- 以 `llman sdd` 命令结果为事实来源。
- 涉及文件/规范变更时执行校验。
- 首选 `llman sdd context` 获取相关 specs，而非全量读取或猜测。
- 当 context 不可用时，按错误提示处理（重建 index 或降级到 `list --specs --json`）。

## Decision Policy
- 高影响歧义必须先澄清。
- 已知校验错误下禁止强行继续。

## Output Contract
- 汇总已执行动作。
- 给出结果路径与校验状态。

## Ethics Governance
- `ethics.risk_level`：按 `low|medium|high|critical` 标注风险等级。
- `ethics.prohibited_actions`：列出绝对禁止执行的动作。
- `ethics.required_evidence`：列出高影响输出前必须具备的证据。
- `ethics.refusal_contract`：定义何时拒答以及安全替代响应方式。
- `ethics.escalation_policy`：定义何时必须升级为用户确认/人工复核。
