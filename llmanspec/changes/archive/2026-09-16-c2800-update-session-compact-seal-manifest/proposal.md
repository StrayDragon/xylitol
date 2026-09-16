---
depends_on:
- c25-fix-compact-overflow-placeholder
needs_specs_change: true
branch: sdd/c2800-update-session-compact-seal-manifest
base_sha: 1cd80374f951f17aee2fb146db3731e3d6f45fd1
base_branch: main
---

# Session JSONL 分段冷备与 compaction seal（manifest + 活跃段，v7）

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

- **布局**：session 从单文件 `{id}.jsonl` 升级为按会话目录保存的 v7 manifest、
  active JSONL 段与有序 sealed cold segments。manifest 是提交指针，不重复承载完整事实。
  compact 成功时把当前 active 中已被摘要的非空前缀写入不可变冷段，active 只保留
  `firstKeptEntryId` 起的尾部及新的 `CompactionEntry`；冷段一旦被 manifest 引用就不得改写。
- **读取**：resume、`load_leaf_branch` 与 session context 构造先读 manifest 和 active 段；
  只有沿当前 leaf 链解析 `parentId`、分支或 bash 配对确实需要时才按索引回读冷段。
  导出与 inspect 仍可显式请求完整逻辑会话，但不再成为 resume 热路径。
- **格式**：session 磁盘版本从 v6 升至 v7。首次访问 v6 单文件时执行一次性迁移：
  先建立 v7 目录并提交 manifest，再清理旧文件；迁移失败保留旧文件以便重试。
  v6 中无法推断的 compaction policy 以 legacy/unknown 标记保留，新写入的
  `CompactionEntry` 必须带 policy/窗口指纹（context window、reserve、keep 预算、
  estimator 版本）。
- **适配**：SessionManager、compaction 写入与 leaf 读取、fork/resume、导出、
  inspect/观测路径发现及 BDD fixtures 同步到目录格式；对外 Driver/wire 方法表不新增
  session 专用旁路。

## Capabilities

- `agent-session-store`：v7 manifest/分段布局、原子提交、v6 一次性迁移、冷段按需读取、
  fork 与恢复语义。
- `domain-compaction`：compact seal 前缀、active/cold 段边界及 policy/窗口指纹。
- `agent-session`：resume 与 compaction-aware history 播种继续使用同一 leaf 分支投影。

## Impact

- **持久化**：新 session 不再以单个 `{id}.jsonl` 作为 SSOT；v7 manifest 引用的段集合才是
  可恢复事实。冷段采用 temp + fsync + rename，manifest 原子替换是可见提交点。
- **兼容**：只对 v6 提供一次性本地迁移；更早或未知版本继续返回可操作的不支持错误，
  不建立长期双读路径。逻辑 JSONL export/import 仍保持可读的条目流，不泄漏 manifest
  控制结构。
- **运行时**：resume 的常规路径避免重新解析全部冷历史；fork 在需要时读取父冷段并把
  选中的逻辑路径复制到子 session，不让子 session 跨目录依赖父段。
- **依赖**：依赖已归档的 `c25-fix-compact-overflow-placeholder`，以其 leaf-only
  compaction 输入与同源估计为前提；收益在实现阶段用现有 lab/回放证据重新测量。

## Decisions

- v6 采用 lazy、幂等的一次性迁移，不提供 v6 只读兼容分支。
- fork 采用「父冷段按需读取 + 子 session 复制选中路径」，不保存跨 session 的段引用。
- 不增加独立的 seal 条数/字节阈值；只有 compaction 产生非空被摘要前缀时才提交冷段，
  避免手动小 session 产生无意义碎片。
