---
depends_on: []
title: "Fix session store transactional integrity"
---

# 会话存储事务完整性修复

## Why

全仓隐式状态机扫描（2026-11-10，静态阅读，见 research/implicit-state-machines.md）发现会话存储插件存在两类可造成**数据丢失 / 读旧历史**的隐式状态机：

1. **压缩封段（seal）四段非原子写**：冷段、sidecar 索引、新 active、manifest 依次落盘，任一步失败只做 remove_paths 回滚；manifest 写成功后、旧 active 删除前崩溃，磁盘同存两代 active，旧段成孤儿且 load 不报错。
2. **段代际（generation）由现存段 max+1 推导**而非持久计数器；manifest 不可读时 current_active_path 静默回退 active-0.jsonl，可能命中上一代残留读到旧历史。

配套问题：落盘布局判定分散六处、leaf 解析回退顺序两处相反、sidecar 损坏静默降级、写锁在「无 manifest」分支被视为已持锁。

## What Changes

- **seal 事务化**：封段全过程先写临时文件，最后单次 manifest 原子替换作为 commit 点；旧 active 删除延迟到 commit 之后且幂等（重启后垃圾回收）。崩溃恢复扫描能判定布局有效或完成清理。
- **generation 持久单调**：manifest 内维护显式 next_generation；废弃 max+1 推导；manifest 缺失 / 不可读时报错而非回退 active-0。
- **布局判定收敛**：单一 layout 探测函数统一 unknown / pending-only / v7 目录 / legacy 单文件 / 孤儿 active 五态；落盘时机（当前仅 assistant 消息触发 flush）改为显式规则：终态 entry 不得长期只存内存。
- **leaf 解析单点化**：统一 resolve_leaf() 承载 hint → 内存 → manifest → anchor 回退链；内存 leaf_ids 区分「缺键」与「显式 None」。
- **sidecar 索引**：损坏时保留降级路径但发出显式诊断（warn + 观测计数），段与索引加轻量校验。
- **写锁**：无 manifest 分支不得返回 Ok 视为持锁；锁文件先于目录判空创建。
- 清理 rider：移除 active_session 死代码；SessionBackend 分支收敛为后端抽象。

## Capabilities

- agent-session-store（会话持久化主契约）；关联 domain-compaction（封段触发侧）。

## Impact

- 范围：`plugins/xylitol-official-plugin-session`（manager 子树）；`crates/xylitol-plugin-sdk`（如需 manifest schema 演进，走可选字段）。
- 兼容：必须能读旧布局（v7 目录 + legacy 拒绝语义不变）；迁移不得要求用户手工清理。
- 验证：崩溃注入测试（每一步 kill）断言布局可恢复；并发 fork/append 压力测试。

## Open Questions

1. seal 事务选型：两阶段意向标记 vs 全临时文件 + manifest 单次 rename（推荐后者，见 research）；是否引入目录级 fsync。
2. 落盘时机：非 assistant 终态 entry（tool 结果、custom）是否立即 flush，还是按批量窗口？
3. legacy 单文件（v6）是否顺带提供一次性迁移而非继续报错？
