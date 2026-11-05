# Tasks

## 1. SDK 图类型与边
- [ ] 新增 EdgeEntry / EdgeKind / SessionEntry::Edge；parse 与 serialize；会话版本升到 8
- [ ] 新增 Branch / Head / ContextView / PrefixEpoch 类型
- [ ] 单测：边排序、墓碑边、环检测

## 2. 投影与 prefix hash
- [ ] build_context(head) 单一入口：反向 DFS + epoch 裁切
- [ ] prefix_hash：规范化 JSON + sha256
- [ ] 单测：同一 epoch 追加尾部 hash 不变；新 epoch hash 变

## 3. v8 manifest 与 COW fork
- [ ] SessionManifest v8：branches / parent ref / workspace_ref
- [ ] fork 时 seal 父 active 快照 + 写 parent ref；子 load 拼接父段
- [ ] fork 边 + branchSummary；保留 parent_session / fork_at_entry_id

## 4. 迁移
- [ ] v7 → v8 迁移器：父链 → parent 边；fork → parent ref + fork 边；compaction → epoch 节点
- [ ] 迁移 dry-run 统计与备份
- [ ] v7 fixture 迁移测试

## 5. 协议与 TUI
- [ ] Command::SessionGraph / TravelSessionGraph；registry 与 wire 映射
- [ ] TUI 树视图改用图数据；fork / travel 语义不变
- [ ] 删除 session_tree / travel_session_tree 名称

## 6. 删除旧路径
- [ ] 删除 fork_inner 复制路径与 clone_entry_with_ids
- [ ] 删除 build_session_tree / tree_prefix
- [ ] 清理旧 fork 复制测试

## 7. Specs 与 BDD
- [ ] 更新 agent-session-store / agent-session / domain-compaction / protocol-app / app-tui-session-tree
- [ ] 新增 BDD：COW fork 引用父条目、墓碑边不进上下文、prefix hash 稳定

## 8. 门禁
- [ ] cargo nextest BDD 全绿
- [ ] just qa 全绿
