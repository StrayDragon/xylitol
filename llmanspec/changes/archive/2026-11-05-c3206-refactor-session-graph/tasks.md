# Tasks

## 1. SDK 图类型与边
- [x] 新增 EdgeRecord / EdgeKind / Branch / ContextView / prefix_hash 类型
- [x] graph.jsonl 解析与写入
- [x] manifest 增加 parent / branches / workspaceRef 字段（formatVersion 保持 7）
- [x] 单测：边排序、墓碑边、环检测

## 2. 投影与 prefix hash
- [x] build_context(head) 单一入口：反向 DFS + compaction epoch 裁切
- [x] prefix_hash：规范化 JSON + sha256
- [x] 单测：同一 epoch 追加尾部 hash 不变；新 epoch hash 变

## 3. COW fork
- [x] fork 时 seal 父 active 快照 + 写 parent ref；子 load 拼接父段
- [x] 子会话追加 branchSummary、labels 与 fork 边
- [x] 内存后端用 parent chain 合并父条目
- [x] fork 初版只写 branchSummary，不做自动 summary / compaction / handoff（c3211）
- [x] 单线 v7 会话仍可读；旧 session 不迁移

## 4. 旧 session 边界
- [x] 旧 session 可丢弃；v6 legacy 仍拒绝；不保留双读双写

## 5. 协议 / TUI
- [x] 延后到 c3210：session_graph / travel_session_graph 与图视图（本 change 不实现）
- [x] 本 change 保持 session_tree / travel_session_tree 兼容单线模式
- [x] 保留 build_session_tree / tree_prefix 供单线 TUI

## 6. 删除旧路径
- [x] 删除 fork_inner 复制路径
- [x] 保留 clone_entry_with_ids 供 append / inject_ids
- [x] 删除无用 helper（edge_count）
- [x] 清理 fork 专用复制测试（已改为 COW 断言）

## 7. Specs 与 BDD
- [x] 更新 specs：fork COW、移除 v7 迁移、移除 protocol/TUI graph 规则
- [x] 更新 BDD：COW fork 引用父条目、墓碑边、prefix hash

## 8. 门禁
- [x] BDD 全绿
- [x] just qa 全绿
