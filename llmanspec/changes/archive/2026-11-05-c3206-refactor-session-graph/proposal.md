---
depends_on: []
title: "Refactor session graph: append-only edges, prefix epochs, copy-on-write fork"
branch: sdd/c3206-refactor-session-graph
base_branch: main
base_sha: d752d77d80de38791c5078545033788872ae84f0
---

## Why

当前会话存储是 per-session 的条目树：fork 把父 leaf 路径逐条复制到子会话（plugins/xylitol-official-plugin-session/src/manager/tree.rs:51-120），上下文只有父链，没有多父边。这带来四个问题：

1. **事实重复**：fork 复制不可变条目并重写 id，历史在子会话里再存一份；fork 成本随历史线性增长。
2. **上下文不可编辑**：只有 leaf 父链，没有显式 context edge；无法表达 ThoughtDAG 式的「连线即上下文」、多父合并、禁用污染子图。
3. **KV cache 前缀没有不变量**：provider 侧有 cache_control / prompt_cache_key，但会话侧没有 epoch 与 prefix hash；compaction / branchSummary / fork 改变前缀是隐式的，测试只覆盖折叠串稳定。
4. **workspace 与调度缺位**：session 只绑定 cwd，没有分支级 workspace commit；没有 trigger 事实，/loop 与 /goal 无处落地。

本 change 把会话存储重构成**会话图**：节点（事实）+ 边（上下文/分叉）+ epoch（前缀边界）+ branch head。fork 改 copy-on-write；上下文投影单一入口并输出 prefix hash；旧 fork 复制路径删除，树视图保留给单线 TUI。workspace 与 scheduler 作为后续 change，在本 change 里只定义它们需要的 branch/workspace 字段。

## What Changes

- **图模型**：Node = 现有 SessionEntry；新增 EdgeRecord（独立 graph.jsonl 追加日志；parent / context / fork；from、to、order、enabled）；Head/Branch 指向图中的一个节点。
- **边只追加**：禁用一条边 = 追加 enabled=false 的边事实；遍历跳过，不删节点。多父按 edge order 排序，缺省按创建序。
- **单一投影**：build_context(head) 反向 DFS 入边，得拓扑序；最新 compaction epoch 截断前缀；返回 ContextView（nodes、epoch_id、prefix_hash、breakpoints）。预览与实发共用这一入口。
- **前缀 epoch**：Compaction 是 epoch 边界；BranchSummary 是 handoff 标记，不截断父前缀；Fork 只写边，不改前缀。prefix_hash = 规范化 JSON 的 sha256；同一 epoch 追加尾部后 hash 不变。
- **fork copy-on-write**：子会话 manifest 引用父会话 sealed 段（fork 时 seal 父 active 快照），不复制条目；子会话头保留 parent_session / fork_at_entry_id；fork 边写入图。
- **fork 初版不做自动压缩**：fork 只写 branchSummary；fork-time summary / compaction / handoff 延后到 c3211，避免触发式压缩失真。
- **manifest 扩展**：记录 branches、edges、parent segment refs、workspace ref；旧 session 可丢弃，不迁移；单线 v7 可读。
- **协议**：c3206 保持 session_tree / travel_session_tree；session_graph 与图视图延后到 c3210。
- **删除**：fork_inner 复制路径；保留 build_session_tree / tree_prefix 供单线 TUI。
- **保留**：manifest/seal 物理层、compaction 摘要与切点、单写者、seq/journal、provider cache 断点。

## Invariants

| 编号 | 不变量 | 落法 |
|---|---|---|
| I1 | 事实只追加 | Edge/Node 只 append；sealed 段不可改写；manifest 原子提交 |
| I2 | 上下文是投影 | build_context 纯函数；预览与实发同一入口 |
| I3 | 前缀 epoch 显式 | epoch 节点 + prefix_hash 测试 |
| I4 | 边不可变 | 禁用追加墓碑边；不删节点 |
| I5 | 单写者 | host slot + write lock；fork 也走同一准入 |
| I6 | workspace 提交只追加 | 本 change 只留 branch.workspace_ref 字段；后端后续 |
| I7 | 外部事件是事实 | 本 change 只留 trigger 字段；scheduler 后续 |

## Shape

| 形态 | 含义 | 本 change |
|---|---|---|
| 会话图 | 节点 + 边 + head | 是 |
| 分支 | head + workspace ref | 是（fork 产生 branch） |
| 工作区 | mount/commit | 否，后续 change |
| 调度 | trigger + /loop /goal | 否，后续 change |

## Capabilities

- agent-session-store：图存储、边条目、COW fork；旧 session 不迁移。
- agent-session：build_context 单一投影、epoch 裁切、prefix hash。
- domain-compaction：compaction 作为 epoch 节点，seal 语义不变。
- layer-architecture：会话图与单写者边界。
- protocol-app / app-tui-session-tree：图协议与图视图延后到 c3210，本 change 不改名。

## Impact

xylitol-plugin-sdk（session 类型 + ports）、xylitol-runtime（session ops + context 播种）、xylitol-official-plugin-session（manager/load/segments/seal/tree）、specs 与 BDD；protocol / serve / TUI 图面延后到 c3210。

## Further Notes

- 调研：llmanspec/changes/c3206-refactor-session-graph/research/architecture-analysis.md
- ThoughtDAG：节点 + 连线 = 上下文；contextOrder 边级排序；单一 buildContext；归档软删除（../thoughtdag/src/lib/graph.ts、src/store/context-builder.ts）。
- tape.systems：append-only + anchor + view；compaction seal 对应 anchor。
- LayerFS：workspace 分支绑定、commit/branch/rewind/merge；后续 workspace change 使用。
- 相关：c3050-add-host-workspace-binding（workspace set）；c3197-remove-plan-mode（已删除模式切换）。

## Open Questions

- fork 语义：保留子 session id（推荐，兼容当前模式）还是同 session 内 branch？
- 跨会话多父 context wire 是否首切片？推荐先只做 fork 单父。
- 图 UI（session_graph）启动时点：c3210。
- 旧 session 丢弃的规模与回滚边界。
