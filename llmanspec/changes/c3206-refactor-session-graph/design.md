# Design — 会话图（Session Graph）

## 目标

- 把 per-session 条目树重构为**节点 + 边 + epoch + head** 的会话图。
- fork 改 copy-on-write：子会话引用父 sealed 段，不复制条目。
- 上下文投影单一入口，输出 epoch_id 与 prefix_hash，钉住 KV cache 前缀不变量。
- v8 manifest 记录 branches / edges / parent refs / workspace ref。
- 协议与 TUI 改用 session_graph；删除旧 fork 复制与树构建。
- workspace 与 scheduler 不在本 change 落地，只留字段。

## 现状

- SessionEntry 变体与 parent_id 树：crates/xylitol-plugin-sdk/src/session/entries.rs:200。
- fork 复制路径并重写 id：plugins/xylitol-official-plugin-session/src/manager/tree.rs:51-120。
- 上下文裁切：entries.rs:354-379（build_context_entries）。
- 投影：crates/xylitol-plugin-sdk/src/llm_project.rs。
- 协议 session_tree：crates/xylitol-protocol/src/registry.rs:102-103；TUI 树：llmanspec/specs/app-tui-session-tree。

## 决策

### 1 图模型

- Node = SessionEntry（Message / Compaction / BranchSummary / Custom / Label / SessionInfo 等）。
- 新增 SessionEntry::Edge(EdgeEntry)：

~~~text
EdgeEntry {
  base: EntryBase,          // id, parent_id, timestamp
  edge_kind: Parent | Context | Fork,
  from: String,             // 源节点 id
  to: String,               // 目标节点 id
  order: u32,               // 多父合并顺序
  enabled: bool,            // 墓碑边为 false
}
~~~

- 边只追加。禁用 = 追加 enabled=false 的新边；遍历取每个 (from,to) 的最新事实。
- Head = manifest 里的 branch.head_entry_id；Branch 记录 id / head / parent_session / fork_entry_id / workspace_ref。

~~~mermaid
flowchart LR
  N1["user"] -->|"parent order=0"| N2["assistant"]
  N3["side question"] -->|"context order=1"| N2
  N2 -->|"parent"| N4["tool result"]
  N4 --> H["head / branch"]
~~~

### 2 投影

build_context(head) 是唯一入口，预览与实发共用：

1. 读取本会话 + parent refs 的条目，建 node map 与入边 map（取每条边最新 enabled 事实）。
2. 从 head 反向 DFS 入边，收集可达节点；按 edge.order 排序，缺省按 timestamp/id。
3. 环检测：发现回边即返回稳定错误（不得静默截断）。
4. 找最新 epoch 节点（Compaction / BranchSummary / Fork），前缀截断到该节点。
5. SessionEntry → AgentMessage → project_for_llm。
6. 计算 prefix_hash，返回 ContextView。

~~~mermaid
sequenceDiagram
  participant UI as 预览
  participant RT as runtime
  participant G as SessionGraph
  UI->>G: build_context(head)
  RT->>G: build_context(head)
  G->>G: DFS 入边 + epoch 裁切
  G-->>UI: ContextView(nodes, epoch, prefix_hash)
  G-->>RT: 同一 ContextView
~~~

### 3 前缀 epoch 与 hash

- epoch 节点：Compaction、BranchSummary、Fork。epoch_id = 节点 id。
- prefix_hash = 规范化 JSON（键排序、去时间戳噪声）的 sha256，覆盖 epoch 之后到当前 head 的投影消息。
- 不变量：同一 epoch 追加尾部消息后 prefix_hash 不变；新 epoch 产生新 hash。
- provider cache 断点仍由方言插件决定（system / tools / last message）；本 change 不改 provider 行为。

### 4 copy-on-write fork

- fork 时把父 active 段 seal 成快照段（含到 fork 切点为止的条目）；父 manifest 更新为新的 active。
- 子 manifest 写 parent ref：parent_session、fork_entry_id、segment refs（相对 sessions root）。
- 子会话 load = 父 sealed 段 + 子 active；entry id 保持父 id，不重写。
- 子会话追加 branchSummary（epoch）与 fork 边；子头保留 parent_session / fork_at_entry_id。
- 父段 sealed 后不可改写；子会话不复制条目。

~~~mermaid
flowchart LR
  PA["父 active"] -->|"fork 时 seal"| PS["父 sealed 快照"]
  PS -->|"segment ref"| CM["子 manifest"]
  CM --> CA["子 active"]
  PS -. "load 时拼接" .-> CM
~~~

### 5 v8 manifest

~~~text
SessionManifest {
  format_version: 8,
  session_id: String,
  branches: Vec<Branch>,
  parent: Option<ParentRef>,
  active_segment: SessionSegment,
  sealed_segments: Vec<SessionSegment>,
  leaf_entry_id: Option<String>,
  workspace_ref: Option<WorkspaceRef>,
}
~~~

- Branch { id, head_entry_id, parent_session, fork_entry_id, workspace_ref }。
- ParentRef { session_id, fork_entry_id, segments: Vec<SegmentRef> }。
- SegmentRef 指向 sessions root 下的相对路径；sealed 段不可改写。
- v7 → v8：父链映射为 parent 边；fork 头映射为 parent ref + fork 边；compaction 映射为 epoch 节点。

### 6 协议与 TUI

- Command::SessionGraph { kind }；Command::TravelSessionGraph { kind, id }；结果含 nodes / edges / branches。
- 删除 session_tree / travel_session_tree 名称，不留双解析。
- TUI 用图数据渲染树形布局；fork / travel 语义不变。

### 7 删除清单

- fork_inner 复制路径与 clone_entry_with_ids。
- build_session_tree 的树构建（保留 graph 构建替代）。
- SessionListEntry.tree_prefix 与列表缩进。
- session_tree / travel_session_tree 协议方法。
- v7 fork 复制相关测试与 spec 场景改写。

### 8 测试

- 单测：边排序、墓碑边、环检测、epoch 裁切、prefix_hash 稳定性。
- BDD：fork 子会话引用父条目（id 相同、无复制）；禁用边后子图不进上下文；同一 epoch 追加尾部 prefix_hash 不变。
- 迁移：v7 fixture → v8，父链/fork/compaction 映射正确。

## 风险

- fork 语义变化可能影响 TUI/协议客户端；改名是 breaking change。
- 跨目录 segment refs 需要 sessions root 解析与安全校验（不得引用 root 外路径）。
- 父 active seal 与 fork 并发：fork 走单写者准入；seal 失败不得留下半成品。
- 迁移大文件耗时；需 dry-run 与可回滚备份。

## 待确认

- fork 保留子 session id（推荐）还是同 session branch。
- 跨会话多父 context wire 是否首切片。
- session_graph 改名是否接受。
