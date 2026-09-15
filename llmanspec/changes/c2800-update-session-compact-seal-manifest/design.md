# Design — Session JSONL 分段冷备与 compaction seal

## 决策摘要

本 change 将事实层从「一个不断增长的 JSONL 文件」改为「manifest 指针 + active
段 + 有序 sealed 段」。段文件仍是 JSONL，条目语义不变；compaction 只改变物理布局
与 context projection，不删除或改写 canonical entry。

- v7 session 根目录为 `<sessions>/<session-id>/`，包含 `manifest.json`、一个 active
  JSONL 段和 `segments/` 下的 sealed JSONL 段。
- manifest 是恢复时唯一的提交指针，不承载完整消息；未被 manifest 引用的临时文件或
  崩溃遗留段不得进入逻辑会话。
- active 段采用 generation 文件名。seal 不覆盖旧 active，而是创建新 active 与新 cold
  段，再原子切换 manifest，保证旧 manifest 仍可恢复。
- v6 的单文件格式只做一次性 lazy 迁移；不建立长期 v6/v7 双读路径。

## v7 物理布局

```text
~/.xylitol/sessions/<session-id>/
├── manifest.json
├── active-<generation>.jsonl
└── segments/
    ├── <generation>-<segment-id>.jsonl
    └── ...
```

manifest 至少记录：

- `formatVersion`、`sessionId`；
- 当前 `activeSegment`；
- 按逻辑顺序排列的 sealed segment 描述（相对路径、generation、首尾 entry id、
  是否包含 session header）；
- 当前 `leafEntryId`，以及供 resume 列表使用的轻量 header/mtime 元数据。

manifest 中的路径只能是 session 目录内的相对路径。首个 active 段含 session header；
首次 seal 后 header 与被摘要前缀进入第一个 cold 段，新 active 从 `firstKeptEntryId`
开始。后续 seal 只移动当前 active 的非空前缀，因此不会重复复制已 sealed 的事实。

`load_entries` 的完整逻辑顺序是「最老 cold 段 → 最新 cold 段 → active 段」；段之间
不要求每段都包含 header。`load_leaf_branch` 不以文件顺序猜 leaf，而使用 manifest
的 leaf 指针与 entry 的 `parentId` 解析当前链。

## Seal 与崩溃提交

compaction 成功后，SessionManager 在单写者约束下执行以下事务：

1. 读取当前 manifest 指向的 active 段，确定 `firstKeptEntryId` 前的非空 prefix、
   保留尾和新 `CompactionEntry`。
2. 将 prefix 写入唯一临时 cold 文件；将保留尾与新 compaction 条目写入新的 active
   临时文件；两个文件都完成写入并 `fsync`。
3. 将两个临时文件 rename 为 generation 固定的最终文件；旧 active 不删除，直到新
   manifest 已提交。
4. 写入 manifest 临时文件、`fsync`，再以 rename 原子替换 `manifest.json`，并尽可能
   同步父目录。
5. 只有 manifest 切换成功后才清理不再需要的旧 active 或 orphan 临时文件。

manifest rename 是可见提交点：崩溃发生在此之前时，旧 manifest 仍指向旧 active；
发生在此之后时，所有新段已经落盘。恢复时只读取 manifest 引用的文件，并将未引用的
orphan 当作可清理垃圾，不把它们拼进 session。active 的普通 append 继续遵守现有
单进程单写者与 flush/sync 纪律；不新增跨进程锁承诺。

只有 prefix 非空且确实有可摘要历史时才 seal。手动 force 在没有可摘要历史时继续
返回既有 Nothing/Already compacted 错误，不创建空 cold 段。

## 读取与 fork

### Resume / context

resume 的热路径先解析 manifest 与 active 段，随后仅为当前 leaf 链缺失的 `parentId`
加载必要 cold 段。建立 `build_context_entries` 所需的逻辑链后，仍沿现有
CompactionEntry 投影规则裁切；不得把所有 cold 段无条件反序列化。

需要 session-wide bash 完成配对时，store 使用 manifest 的轻量索引定位相关段，按需
加载命中的段；不能为了 done 集合重新扫描全部冷历史。完整导出、诊断 inspect 和显式
历史统计可以请求全量逻辑条目，并明确属于冷路径。

### Fork / tree

父 session 在 cold 段中的切点通过 segment resolver 按需读取。fork 仍把选中的逻辑
路径复制到子 session 并重新链接 `parentId`，子 session 不保存对父目录或父 segment
的跨 session 引用。子 session 首次创建为 v7 active 段，按既有 deferred assistant
flush 规则处理空子分支。

### v6 lazy migration

访问 `<sessions>/<id>.jsonl` 且不存在 v7 manifest 时：

1. 严格解析 v6 JSONL，拒绝更早或未知版本；
2. 复制条目到 v7 active 临时段，将 header version 升为 7；
3. 对没有历史 policy 的 CompactionEntry 写入 legacy/unknown 标记，不伪造当时的
   context window 或 token budget；
4. 原子提交 manifest；
5. 成功后删除旧单文件；若删除前进程退出，下次访问以 manifest 为准并完成清理。

迁移必须幂等。任何解析、写入或 manifest 提交失败都保留 v6 原文件，并向调用方返回
可操作错误；不得留下一个看似 v7 但 manifest 不完整的 session。

## CompactionEntry policy 指纹

`CompactionEntry` 增加可选的 `policy` 快照，以便兼容迁移来的历史条目：

```text
policy:
  contextWindow: u64
  reserveTokens: u64
  keepRecentTokens: u64
  estimatorVersion: string
```

新执行的 compaction 必须写入完整 policy 快照；迁移的 v6 条目可以为 unknown/legacy，
并且该标记不得被当作当前配置的可信指纹。policy 是条目事实的一部分，后续改模型、
窗口或估算器时，诊断可明确说明摘要是在何种预算下产生，而不改变现有
`summary`、`firstKeptEntryId`、`tokensBefore` 与 `details` 语义。

## 适配边界

- `protocol::session` 只增加 v7 manifest/segment 元数据与 compaction policy 的共享
  serde 词汇；不把文件系统实现放入 protocol。
- `infra::session::SessionManager` 负责 manifest、segment resolver、迁移、原子 seal
  和完整逻辑条目组装；现有 `XySessionStore` seam 继续承载 agent/compaction 调用。
- agent compaction 只负责把 policy 快照随 CompactionEntry 写出并调用 store 的
  segment-aware append/seal 能力，不另建旁路存储。
- export/import 输出逻辑条目 JSONL，不把 manifest 当作用户会话条目；resume 面板、
  inspect 与 obs 路径发现以 manifest/session directory 为路径锚点。
- 不新增 wire Command/Event；目录布局是本地持久化实现细节。

## 规格与验证落点

- `agent-session-store.feature`：v7 目录与 manifest、原子可恢复性、v6 迁移、冷段
  不可变性、按需 leaf/fork 读取。
- `domain-compaction.feature`：seal 后 active/cold 边界与新 CompactionEntry policy
  指纹。
- BDD 复用 `SessionManager` / `compact_session` 的现有 fixture 与 binding；manifest
  纯解析、崩溃注入和 policy 序列化细节由现有 infra/compaction 单测补足。

## 风险与缓解

- **迁移失败或磁盘空间不足**：旧 v6 文件保留，v7 manifest 只有完整提交后才可见；
  错误携带 session id 与恢复动作。
- **冷段解析仍过宽**：segment descriptor 与 parent/entry 索引必须先筛选，再加载
  JSONL；测试断言 resume 不触发全量 cold scan。
- **policy 历史未知**：迁移条目标记 legacy/unknown，不把缺失指纹静默解释为当前
  配置；新条目强制完整快照。
- **路径适配遗漏**：export、resume、inspect、obs discovery 各保留一个端到端回归场景，
  并在删除旧 `.jsonl` 假设前完成 expand-contract 迁移。
