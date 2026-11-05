# 会话图与不可变前缀：架构分析

> 调研 2026-11-05。对照：tape.systems（append-only tape + anchor + view）、ThoughtDAG（节点 + 连线 = 上下文）、LayerFS（ephemeral workspace + commit/branch）。本文件是 c3206 的 change 作用域调研，不是 live specs。

## 结论

当前架构的强项是 append-only 会话条目、manifest/seal 原子提交、单写者、协议信封与 provider 前缀缓存。缺口有四个：fork 复制条目而不是引用；上下文只有父链、没有多父边；KV cache 前缀没有显式 epoch 不变量；workspace 只有 cwd、没有分支级提交。建议以**会话图（节点 + 边 + epoch）**为稳定基础，fork 改 copy-on-write，workspace 与 scheduler 作为后续 change。

## 1 现状

### 1.1 存储与 fork

- v7 会话目录 = manifest + active JSONL + sealed cold JSONL；manifest 是恢复 SSOT（agent-session-store r1102）。
- 条目以 parent_id 形成会话内树；SessionEntry 变体含 Message / Compaction / BranchSummary / Custom / Label / SessionInfo 等（crates/xylitol-plugin-sdk/src/session/entries.rs:200）。
- fork 把父 leaf 路径逐条复制到子会话，重写 id 与 parent_id，再追加 branchSummary（plugins/xylitol-official-plugin-session/src/manager/tree.rs:51-120；spec r1110）。
- 子会话头记录 parent_session 与 fork_at_entry_id；子会话不引用父段文件（r1110）。

~~~mermaid
flowchart LR
  P["父会话<br/>entries 1..N"] -->|"fork 复制路径"| C["子会话<br/>entries 1..K + branchSummary"]
  P -. "parent_session/fork_at" .-> C
~~~

### 1.2 上下文投影

- leaf 分支 → build_context_entries：取最新 CompactionEntry，保留摘要 + firstKeptEntryId 起尾部（entries.rs:354-379）。
- SessionEntry → AgentMessage → project_for_llm，Env 折叠串形状稳定（llm_project.rs；agent-session r1084）。
- provider 侧：Anthropic 在 system / tools / last message 打 cache_control；OpenAI 发 prompt_cache_key（anthropic_messages.rs:132-196；openai_completions.rs）。
- 现有测试只断言折叠串稳定与投影快照，没有「同一 epoch 追加尾部后前缀 hash 不变」的会话级测试。

### 1.3 运行时与协议

- 单写者：一 runtime 一 session；busy 拒绝第二次根提交（src/AGENTS.md:170）。
- seq/journal：事实事件进 journal，流式 tape 不写 journal、不耗 seq；环形 1024；环绕发 resync（server-core r1798/r1806/r1807；ws.rs:15-90）。
- workspace：host 从 payload 取 cwd，按路径物化 driver；没有 worktree / overlay / commit（host.rs:881-945）。
- loop：七阶段中间件；没有 /loop 调度器，也没有 /goal 状态。

## 2 不变量分级

| 编号 | 不变量 | 现状 | 目标 |
|---|---|---|---|
| I1 | 事实只追加 | v7 append + seal；fork 复制 | 图节点/边只追加；fork 引用 |
| I2 | 上下文是投影 | leaf 链 + compaction 裁切 | 图遍历 + epoch 裁切 |
| I3 | 前缀 epoch 显式 | 只有 provider cache 断点 | 会话级 epoch + prefix hash |
| I4 | 边不可变 | parent_id 树 | 边是事实；禁用追加墓碑边 |
| I5 | 单写者 | 已有 | 保留，扩展到 branch |
| I6 | workspace 提交只追加 | 无 | workspace port + commit |
| I7 | 外部事件是事实 | 无调度 | trigger 事件 + /loop /goal |

## 3 对照

- **tape**：append-only + anchor（读边界）+ view（选择器）。我们的 compaction seal 已是 anchor；缺的是把 anchor 显式成 epoch，并让 view 可编辑。
- **ThoughtDAG**：节点 + 连线；连线带 contextOrder / contextDepth；单一 buildContext 同时喂预览与实发；归档是剪掉但保留、遍历跳过。可借鉴：边级排序、单一投影、语义快照哈希、软删除。
- **LayerFS**：每个 agent 挂一个可写 Workspace 到共享提交历史；commit/branch/rewind/merge；append-only objects。可借鉴：workspace 分支绑定、提交去重、并发冲突拒绝。

## 4 目标模型

~~~mermaid
flowchart TB
  subgraph G["SessionGraph（只追加）"]
    N1["Node: user"]
    N2["Node: assistant/tool"]
    N3["Node: compaction (epoch)"]
    E1["Edge: parent"]
    E2["Edge: context wire"]
    H["Head / Branch"]
    N1 --> E1 --> N2
    N1 --> E2 --> N3
    N2 --> H
  end
  H --> P["build_context(head)"]
  P --> V["ContextView<br/>ordered nodes + epoch + prefix_hash"]
  V --> L["LLM request<br/>stable prefix + tail"]
  H --> W["WorkspaceRef<br/>local / worktree / LayerFS"]
  H --> S["Schedule / Goal"]
~~~

### 4.1 节点与边

- 节点 = 现有 SessionEntry；新增 Edge 条目（kind=parent/context/fork，from/to，order，enabled）。
- 边只追加；禁用一条边 = 追加一条 enabled=false 的边事实；遍历跳过，不删节点。
- 多父：一个节点可有多条入边；投影按 order 排序，缺省按创建序（ThoughtDAG contextOrder 的对应物）。

### 4.2 投影

- 从 head 反向 DFS 入边，得拓扑序；遇到最新 epoch 节点截断前缀。
- 单一 build_context(head) 同时服务预览与实际发送，避免漂移。
- 返回 ContextView { nodes, epoch_id, prefix_hash, breakpoints }。
- prefix_hash = 规范化 JSON 的 sha256；测试断言同一 epoch 追加尾部后 hash 不变。

### 4.3 fork copy-on-write

- fork 不再复制条目：创建子 branch head + fork 边，引用父 head 的不可变前缀。
- 父 active 段在 fork 时 seal 成快照（或复制该段），子会话 manifest 引用父段路径；sealed 段不可改写。
- 子会话仍保留 parent_session / fork_at_entry_id，产品语义不变（fork 仍产生子 session id）。

### 4.4 workspace 与 scheduler（后续 change）

- workspace port：mount / commit / branch / merge / rewind；后端 local dir / git worktree / LayerFS。
- branch 绑定 workspace commit；tools 在 mount 内运行。
- scheduler：timer / channel / user 事件作为 trigger 事实；/loop 与 /goal 是策略，不是隐藏状态；admission 复用单写者。

## 5 迁移与移除

- v7 → v8：读旧会话，父链映射为 parent 边；fork 头映射为 fork 边；compaction 映射为 epoch 节点。
- 保留 manifest/seal 作为物理层；新增 graph 索引（边/分支/workspace ref）。
- 移除：fork_inner 复制路径、build_session_tree 树构建、tree_prefix 列表缩进、session_tree 协议方法（改名 session_graph）。
- 旧会话一次性迁移；不做双读双写，不留兼容 shim。

## 6 风险与待确认

- fork 语义：子 session 还是同 session 内的 branch？建议保留子 session id（兼容当前模式），图在 session 之间建立 fork 边。
- 跨会话多父：先只支持 fork 单父；跨会话 context wire 后续。
- 迁移规模：现有会话数量与段大小未知；需在 change 里做 dry-run 统计。
- 协议改名：session_tree → session_graph 是 breaking change；TUI 同步更新，不留双解析。
- LayerFS 集成：平台限制（Linux/macOS via DSH、Windows WSL2）、进程边界、权限模型需单独评估。
