# Design — 上下文注入框架插件（v7 SSOT 对齐版，全部已决 2026-11-11）

## 0. 架构

~~~mermaid
flowchart LR
  subgraph SRC["注册三来源"]
    B["内置（代码注册）"]
    C["用户配置 context_inject 节<br/>global→project→user 后层覆盖"]
    P["插件 ContextInjectorRegistry"]
  end
  SRC --> REG["注册表"]
  REG --> CO["协调器：7 点调度<br/>priority→id 字典序（确定性）"]
  CTX["内核 BeforeStepCtx<br/>+3 只读字段（唯一 core 改动）"] --> CO
  SDK["plugin-sdk：run_command_capture<br/>（命令物化共享助手，复用 host_env）"] --> CO
  CO --> OUT["HistoryRow（该 turn 用户输入之前）<br/>/ Ephemeral；Dedup/TTL/group"]
~~~

依赖清单（零插件间依赖）：plugin-sdk（词表 + 命令助手）、kernel core LoopMiddlewareRegistry。不依赖 hooks 插件、事件总线。

## 0.5 上游术语 gate

本 change 假定 c3240 已合入。若下列名词在仓库不存在，先检查上游是否完整实现（llman-sdd show c3240；rg 'BeforeStep|has_new_user_messages|StepStart'），勿自行造词：LoopStage::BeforeStep / AfterStep、BeforeStepCtx.has_new_user_messages（c3240 自 DecideContinueCtx 同义迁入）、StepStart / StepEnd 事件、run_start / run_end / step_start / step_end hook 事件名（run_* 为宿主实现术语命名，非概念层——若见把 run 当概念层的表述，对照 c3240 §0 三层版修订）。勘误（2026-11-11）：has_new_user_messages 现状仅在 DecideContinueCtx，BeforeStep 侧字段由 c3240 T2 迁入——本 change 内核暴露 = 3 个资源字段，边界标志为 c3240 交付物。勘误（2026-11-11）：has_new_user_messages 现状仅在 DecideContinueCtx，BeforeStep 侧字段由 c3240 T2 迁入——本 change 内核暴露 = 3 个资源字段，边界标志为 c3240 交付物。

## 1. 注入点与编译表（SSOT 对齐）

| 注入点 | 既有同名事件 | 编译到 | 谓词 |
|---|---|---|---|
| session_start | hook session_start | 首个 turn_pre 评估 | first_of_session（新开基线 / resume 差分） |
| session_shutdown | hook session_shutdown | 下一次 session_start 补记 | 持久行延迟落账 |
| run_start | hook run_start（c3240 自 agent_start） | turn_pre | first_of_run |
| run_end | hook run_end（自 agent_end）；run_settled 为观察专用不设点 | step_end | is_settle（Dedup 幂等；精确 run-end 待消费者） |
| turn_pre | 无既有（唯一新名） | step_start | has_new_user_messages |
| step_start | hook step_start（c3240 自 turn_start） | LoopStage::BeforeStep | 恒真 |
| step_end | hook step_end（自 turn_end） | LoopStage::AfterStep | 恒真 |

step 之间不评估不注入（工具表冻结无漂移、skills 顺延下一 turn_pre）。

## 2. 既有五机制映射（合理性证据）

| 机制 | 点 | 放置 | 去重 |
|---|---|---|---|
| session_env | turn_pre + 压缩后 ensure | HistoryRow | SuppressIfUnchanged |
| session_prepare | turn_pre + 压缩后 | HistoryRow | SuppressIfUnchanged |
| runtime_policy | run_start / Event | SystemPromptFragment（留 core） | FragmentIdSet |
| skills $name 展开 | step_start（引用） | Ephemeral | by-name |
| drift（新） | turn_pre（含首评） | HistoryRow（group: system-reminder） | SuppressIfUnchanged(指纹集) |

## 3. 注册与配置

- 内置：代码注册（drift 等）。插件：ContextInjectorRegistry Ctx 服务（ReloadHub 同款），三步接入。
- 用户配置样张：

~~~yaml
context_inject:
  injectors:
    - id: git-status
      point: turn_pre                  # session_start|session_shutdown|run_start|run_end|turn_pre|step_start|step_end
      command: "git status --short --branch"
      placement: ephemeral              # history_row | ephemeral
      dedup: suppress_if_unchanged      # none | latest_wins | suppress_if_unchanged
      timeout_secs: 2
      group: null
~~~

规则：三层同 id 后层覆盖（hooks 同款）；两阶段校验（未知 point / 重复 id 报错）；命令物化 stdout 截断 4KB、默认超时 2s、失败跳过 + warn（advisory fail-open）；project 层随 trust 门禁，command 关联 c3330 审批门（未来）。

## 4. 协调器与 drift 注入器

- 调度：7 点 ×（priority 默认 0 → 同优先级 id 字典序）；空注册零开销。
- 放置：HistoryRow 经 RuntimeSessionStore 持久化（该 turn 用户真实输入之前）；Ephemeral；Dedup / TTL / group 分组聚合（同组单行）。
- drift：三源指纹（skills hash + stat 快路径；两指纹直读 ctx）；快照 CustomEntry（latest-wins，压缩保留）；铁律 = 注入只由边界差分决定，watch 只加速。行样张：

~~~text
<system-reminder>
[skills] skill "B" 内容已变更（3f2a… → 91c4…），本轮展开将更新；此前对话中对 B 的引用可能已过期
[tools] 工具清单已变更（8d10… → bb37…）：禁用 ffgrep / fffind（capability 覆盖）
</system-reminder>
~~~

## 5. spec delta（landing 时）

- agent-prompt：框架 requirement（7 点 SSOT 命名 / 编译语义 / 三来源 / 排序 / 空注册零开销）+ context_reminder 实例条款。
- runtime-config：context_inject 节 schema、合并、校验。plugin-sdk：run_command_capture 助手契约。
- agent-llm-projection：HistoryRow 投影。loop 缝：ctx 字段。workspace-assembly：默认装配 +1。domain-compaction：快照保留 +1。domain-security：project 层 trust 边界。

## 6. 测试边界（seam）

- 单测：7 点编译矩阵；三来源与三层覆盖；排序确定性；命令物化护栏（截断 / 超时 / 失败跳过）；drift 边界矩阵；分组聚合；空注册零开销；快照压缩保留；命名 SSOT 对照（点值 = hook 事件名）。
- BDD：agent-prompt（skill B→B1 下一 turn_pre 可见；resume 离开期间变更注入）；runtime-config（校验与合并）。

## 7. 风险与回滚

- 风险：run_end / session_shutdown 为编译近似（Dedup 兜底），出现精确消费者再加缝；配置命令执行面（trust + c3330 + 护栏）。
- 回滚：插件可整体卸载；core 侧仅 3 个无害字段；配置节未配置零装配。
