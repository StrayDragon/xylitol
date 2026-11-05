# 会话图走读：从一条消息到一次 fork

## 0 对齐一句话

c3206 只做**基建**：节点、边、head、epoch、单一投影、COW fork。它不是 ThoughtDAG 的 UI 复刻，也不是 tape 的复刻；只借数据模型与读边界概念。

## 1 术语

- **节点**：一条事实（用户消息、助手消息、工具结果、压缩摘要、分支摘要）。
- **边**：上下文关系（parent / context / fork），只追加；禁用一条边 = 追加 enabled=false 的新边。
- **head**：当前读起点；从 head 反向走边得到上下文。
- **epoch**：前缀边界（compaction）；投影在最新 compaction 处裁切。branchSummary 是 handoff 标记，不截断父前缀。
- **view**：从 head 反向走边得到的消息序列。
- **prefix_hash**：view 的规范化 JSON 指纹（键排序、时间戳归零）。
- **anchor**：持久化的读边界标记；把默认读取起点移到新位置，不删除历史。LayerFS 没有同名概念，最接近 Branch head（读起点）与 Commit（不可变快照）；为避免与 workspace commit 冲突，保留 anchor。

## 2 一个具体例子

会话 S，用户先修测试，压缩后追问，再分叉。

~~~mermaid
flowchart LR
  u1["u1 用户: 修好 failing test"] --> a1["a1 助手: read src/lib.rs"]
  a1 --> t1["t1 工具结果: ..."]
  t1 --> a2["a2 助手: 找到原因"]
  a2 --> c1["c1 压缩摘要 (epoch)"]
  c1 --> u2["u2 用户: 顺便更新文档"]
  u2 --> a3["a3 助手: 已更新"]
  a2 -. "context 边 order=0" .-> q1["q1 为什么测试失败?"]
~~~

### 2.1 从 head 投影

head = a3。反向走边得到 u1,a1,t1,a2,c1,u2,a3；最新 epoch 是 c1，裁切后只留 c1,u2,a3。

~~~mermaid
flowchart TB
  H["head = a3"] --> R["反向走边"]
  R --> O["u1,a1,t1,a2,c1,u2,a3"]
  O --> E["最新 epoch = c1"]
  E --> C["裁切: c1,u2,a3"]
  C --> P["投影消息: 摘要 + u2 + a3"]
  P --> X["prefix_hash"]
~~~

### 2.2 多叉思考

q1 是一条 side question，只加一条 context 边 a2 → q1，head = q1。反向走边得到 u1,a1,t1,a2,q1；没有 epoch，全部保留。

如果 a1/t1 是污染子图，追加 t1 → a2 的 enabled=false 边；从 q1 再投影时跳过 t1 及其上游，但节点仍在图中，不删历史。

~~~mermaid
flowchart LR
  u1 --> a1 --> t1
  t1 -. "enabled=false 墓碑边" .-> a2
  a2 --> q1
  a2 --> c1
~~~

### 2.3 fork

在 a2 分叉：写 fork 边 a2 → b1，b1 是 branchSummary（handoff 标记，不是 epoch）；子会话引用父 sealed 段，不复制条目。子 head = b2 时，上下文 = 父前缀到 a2 + b1 摘要 + b2。

初版只写 branchSummary，不做自动 summary / compaction / handoff；fork-time 上下文缩减延后到 c3211，避免触发式压缩失真。

~~~mermaid
flowchart LR
  a2 -->|"fork 边"| b1["b1 branchSummary (handoff)"]
  b1 --> b2["b2 新分支消息"]
  P["父 sealed 段"] -. "子 manifest parent ref" .-> b1
~~~

### 2.4 KV cache

请求 1 的前缀是 system + tools + c1 + u2 + a3；provider 缓存该前缀。请求 2 追加 a4，前缀逐字节不变，prefix_hash 不变，命中缓存。

发生新压缩 c2 时，epoch 改变，prefix_hash 改变；这是预期的 cache miss，不是 bug。

~~~mermaid
sequenceDiagram
  participant R1 as 请求 1
  participant R2 as 请求 2
  participant P as provider
  R1->>P: system + tools + c1 + u2 + a3
  P-->>R1: cache write
  R2->>P: 同前缀 + a4
  P-->>R2: cache hit
~~~

## 3 /loop 与 /goal

- **/loop** 是执行方式：每隔一段时间或条件触发一次 run，反复做同一件事，直到手动停或停止条件满足。
- **/goal** 是任务定义/终止标准：围绕一个目标规划步骤、持续迭代，直到完成或失败。
- 两者可以配合：/goal 定目标，/loop 反复检查/推进。

~~~mermaid
flowchart TB
  G["Goal 事实: tests green"] --> M["中间件: after_tool / decide_continue"]
  L["Loop 事实: every 5m"] --> S["Scheduler: trigger"]
  S --> R["Run"]
  R --> M
  M -->|"未达成"| L
  M -->|"达成"| D["goal.done"]
~~~

这两项都依赖同一套基建：branch 身份、事件事实、单写者准入。它们不在 c3206 内实现。

## 4 对齐表

| 能力 | c3206 | 后续 |
|---|---|---|
| 节点/边/head/epoch | 已落地 | — |
| 单一投影 + prefix_hash | 已落地 | 实际模型请求接线后续 |
| fork 边 + branch 元数据 | 已落地 | COW 引用父段后续 |
| /loop | 不做 | delayed draft / issue |
| /goal | 不做 | delayed draft / issue |
| workspace（worktree/LayerFS） | 只留 WorkspaceRef 字段 | 独立 workspace change |
| nsh GUI / 图可视化 | 只留图数据 | 独立 UI change |
| tape anchor | 只借读边界概念 | 不复制 tape |

## 5 日程现状

- 当前 roadmap #28 没有 /loop、/goal、workspace、图 UI 的 FR。
- #19「Loop 管理与触发可视化」已关闭；/loop 需要新 FR 或 delayed draft。
- #71「Guarded Decision Reuse」相关但不是 /loop；不能替代调度。
- 建议：c3206 保持基建主线；/loop、/goal、workspace、图 UI 以 delayed draft 或 GitHub issue 停放，兑现时再开 change。
