---
depends_on:
  - c3240-fix-react-loop-run-state
title: "Add context injection framework with drift reminders"
---

# 上下文注入框架（官方插件）与首个漂移提醒注入器

> 2026-11-11 定稿 v7（SSOT 对齐版）：注入点命名复用统一生命周期词表，放弃 pre/post 对称网格；零插件间依赖；命令物化经 plugin-sdk 共享助手。词汇：session ⊃ turn（问答轮）⊃ step（run 为宿主实现术语）；依赖 c3240 缝名与事件更名（step_* / run_*）。

## Why

仓库已有五个各自为政的注入机制（session_env / session_prepare / runtime_policy 片段 / skills $name 展开 / 未来的漂移提醒），同一问题五套写法（映射见 design §2）。缺口：无统一框架、无用户可注册入口、resume / 新 session 非一等 checkpoint（归约可解）、skills 内容漂移与工具表变更无提醒。

## What Changes

- **注入点 = 统一生命周期名（SSOT 对齐，7 枚）**：session_start / session_shutdown / run_start / run_end / turn_pre / step_start / step_end。复用既有 hook 事件名（session_*、step_*；run_* 随 c3240 自 agent_* 更名）；turn_pre 为唯一新名（问答轮边界不存在于任何既有词表）；turn_post 不设（= step_end + is_settle，注入器以 Dedup 表达）。编译到既有缝零新内核钩子（编译表见 design §1）。
- **注册三来源**：内置（代码注册）；插件（ContextInjectorRegistry Ctx 服务）；用户配置（context_inject 节：id / point / command / placement / dedup / timeout_secs；三层 global→project→user 同 id 后层覆盖；两阶段校验）。
- **零插件间依赖**：缝仅 kernel core 的 LoopMiddlewareRegistry（BeforeStep / AfterStep）；不依赖 hooks 插件或事件总线。配置注入器的命令物化走 plugin-sdk 共享助手 run_command_capture（复用 host_env 消毒 SSOT；hooks 插件后续顺路迁移同一助手，非强制——消灭双实现漂移病根）。失败 / 超时跳过注入 + warn（advisory fail-open，与 c3235 强制 hooks fail-closed 对照）；输出截断 4KB、默认超时 2s。
- **内核暴露（唯一 core 改动）**：BeforeStepCtx 增 3 个只读字段（skills 快照 / tools_fingerprint / prepare_fingerprint）；turn 边界判定复用 has_new_user_messages。
- **框架 = 官方插件 xylitol-official-plugin-context-inject（默认装配）**：7 点调度；排序确定性 = priority → id 字典序；Placement = HistoryRow（持久 Env 行，注入于该 turn 用户真实输入之前）+ Ephemeral；Dedup / TTL / group 分组聚合。
- **首个注入器 = drift**：三源指纹、快照 CustomEntry（latest-wins，压缩保留）、watch 只加速铁律、system-reminder 单行分组、无变不注入。
- **仍留 core**：SystemPromptFragment 放置；既有三族行不动。custom_type 与服务名查无冲突（context-fingerprint-snapshot / ContextInjectorRegistry）。

## Capabilities

- agent-prompt（框架 + context_reminder 实例条款）；agent-llm-projection；runtime-config（context_inject 节）；plugin-sdk（共享命令助手）；loop 缝；runtime-resource-discovery；domain-compaction；workspace-assembly；domain-security（project 层 trust 边界）。

## Impact

- 范围：kernel/runtime（ctx 3 字段）、plugin-sdk（命令助手）、新插件 crate、runtime-config（新节）、compaction 插件（保留 +1）。依赖 c3240 合入（缝名与 step_*/run_* 事件更名）。

## 决策记录（2026-11-11，全部已决）

1. （已决）框架落官方插件（默认装配）；内核暴露 = BeforeStepCtx 3 只读字段，零新钩子。
2. （已决 v7）注入点命名对齐统一生命周期 SSOT：session_start / session_shutdown / run_start / run_end / turn_pre / step_start / step_end；turn_pre 唯一新名；放弃对称网格。
3. （已决）注入时机 = turn_pre（has_new_user_messages 收窄）；step 之间不评估。
4. （已决）注册三来源（内置 / 插件服务 / 用户配置三层合并）。
5. （已决 v7）零插件间依赖；命令物化 = plugin-sdk 共享助手 run_command_capture（hooks 顺路迁移非强制）。
6. （已决）配置注入器：4KB 截断、2s 默认超时、失败跳过 + warn（advisory fail-open）。
7. （已决）排序确定性：priority → id 字典序，注册序不参与。
8. （已决）drift 为首个注入器；快照压缩保留；watch 只加速。
9. （已决）Placement = HistoryRow + Ephemeral；SystemPromptFragment 留 core；既有三族不迁移。
10. （已决）project 层随 trust 门禁；command 执行关联 c3330 审批门（未来）。
11. （已决 v7）agent_* → run_* hook 事件更名归 c3240（宿主实现术语收口，run 不进概念词汇）。
