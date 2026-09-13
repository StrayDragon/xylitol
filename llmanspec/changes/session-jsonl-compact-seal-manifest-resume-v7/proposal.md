---
depends_on:
- c25-fix-compact-overflow-placeholder
needs_specs_change: true
rules_touched:
- sr-v6
---

# session JSONL 分段冷备（manifest + 活跃段，v7）

## Why

session 磁盘格式是**单文件 append-only JSONL**，compaction 只追加 `CompactionEntry` 不删前缀
（`src/agent/compaction/mod.rs:336-338`）。实测副作用：

- 重复压缩场景下 summary 条目持续累积（babb2fbc 6 分钟 7 条、92fa9adf 9 条 / 290KB）；
- 每次 run（含 resume 后首轮）`load_conversation_history` 全文件解析 JSONL
  （`src/agent/capabilities/session_ops.rs:100-129`），冷历史反复重读。

用户提案：**compact 前的内容及时 archived（冷备），下一次 resume 只加载当前指向的 session 段**。

与 maka 对照（`docs/research/maka-runtime-kernel-lessons-2026.md`）兼容：maka 不变量 3
「Compaction 改投影不改历史」要求 canonical log 不被改写——分段冷备**不改写任何事实**，
只是把 `[header, first_kept)` seal 成不可变冷段，属于「事实层布局」变更（maka 前置表：
需 session 格式 change 立项 + 单一 canonical 写入路径）。

附带结论：CompactionEntry 无 policy/窗口指纹——换模型 / 换窗口后旧压缩决策是否仍合理无从判断
（maka 开放问题 2 的实证：32k 小窗口模型上压缩空转溢出）。v7 一并补该字段。

## What Changes

- **布局**：session 从单文件 `{id}.jsonl` → 目录 + manifest 指针。compact 成功时把
  `[header, first_kept)` seal 为**不可变冷段文件**（temp+rename 原子提交 manifest），
  活跃段只保留尾部；冷段一旦写就 MUST NOT 再改写。
- **读取**：resume / `load_leaf_branch` 只读 manifest + 活跃段；跨冷段回读（如 fork 树
  `parentId` 指向冷段内条目）按需回读冷段。
- **格式**：版本 v6 → v7（Pre-0.0.1 无兼容债，直接升级）；`CompactionEntry` 增加
  policy/窗口指纹字段（context_window / reserve / keep 预算 / estimator 版本）。
- **适配**：resume 面板、inspect 观测脚本、obs 路径发现等既有读方同步。

## Open Questions

- v6 旧文件策略：一次性迁移写 v7，还是 v6 只读兼容（推荐前者，Pre-0.0.1 纪律）。
- fork 时 `parentId` 跨冷段：按需回读 vs fork 时复制前缀到新 session。
- 是否要 seal 阈值（冷段最小体积 / 条数），避免小 session 频繁建目录碎片。
- **依赖 c25 落地后重估收益**：压缩空转根因修复后，summary 累积速率大幅下降，
  分段冷备的性能/磁盘收益需按新基线重新度量，再决定是否值得动格式。
