# Tasks — c2800-update-session-compact-seal-manifest

## 测试 seam（复用既有 harness，不新发明）

1. **SessionStore BDD seam**：复用
   `llmanspec/specs/agent-session-store/agent-session-store.feature`、
   `tests/bdd/bindings_agent_session_store.rs`、`steps_session.rs` 与
   `XySessionStore`/`SessionManager` fixture，覆盖 manifest、v7 active/cold 段、
   v6 lazy migration、按需 leaf/fork 读取与删除/list。
2. **Compaction BDD seam**：复用
   `llmanspec/specs/domain-compaction/domain-compaction.feature`、
   `tests/bdd/bindings_domain_compaction.rs`、`steps_compaction.rs` 与既有 fake model，
   覆盖 seal 后 active tail 和 `CompactionEntry` policy 指纹。
3. **纯数据/崩溃边界单测**：复用 `protocol::session` serde/parse 测试、
   `infra::session::manager` persist/load/tree 测试以及 `agent::compaction` 既有
   in-memory `SessionManager` 测试；验证 manifest 原子切换、orphan 忽略、legacy
   policy 标记和 segment resolver，不把这些细节凭空扩成新的公共 seam。
4. **适配回归**：复用既有 export/import、resume list、obs/inspect 路径发现测试；
   JSONL export 继续断言逻辑条目流，不断言 manifest 内部结构。

## 实施任务

### T1 protocol：v7 持久化词汇与 policy 快照

- 在 `protocol::session` 定义 v7 manifest/segment 元数据
  的 serde 词汇与 `SESSION_VERSION = 7`；保持 protocol 不依赖文件系统。
- 为 `CompactionEntry` 增加可选的 policy 快照字段，字段使用 camelCase：
  `contextWindow`、`reserveTokens`、`keepRecentTokens`、`estimatorVersion`；
  新 compaction 要求完整快照，legacy/unknown 仅用于迁移条目。
- 更新 parse/version 单测：v7 round-trip、未知/更早版本拒绝、旧 policy 缺省与
  legacy 标记不被误当作当前指纹。
- 绑定后同步 `agent-session-store` 的版本条款与 `domain-compaction` 的条目
  条款；本任务不在绑定前编辑 live specs。

### T2 infra：创建、append 与 manifest 基础生命周期

- [blocked-by: T1] 在 `SessionManager` 建立 per-session directory、
  manifest、generation active 段与 sealed segment descriptor 的读写模型。
- 迁移 `create`、普通 append、deferred assistant flush、`exists`、`list`、
  `delete` 与 `get_session_file`/等价路径，使新建 v7 session 不再产生单一
  `{id}.jsonl` SSOT；保留 in-memory backend 语义。
- manifest 路径校验只允许 session root 内相对路径；单写者、flush/sync 与
  不引入跨进程锁的既有纪律保持不变。
- 单测覆盖新建/追加/重载、pending-only session、list 顺序、删除整棵 session
  目录和旧 `.jsonl` 不被误列为 v7 目录。

### T3 infra：v6 lazy migration 与兼容收口

- [blocked-by: T2] 首次发现 v6 `{id}.jsonl` 时严格解析并构造 v7 active
  段与 manifest；manifest 成功提交后再清理旧文件。
- 迁移过程幂等：失败保留 v6 原文件；重复访问优先 v7 manifest；v6 compaction
  policy 写入 legacy/unknown，不补造窗口或预算。
- 单测覆盖迁移成功、迁移中断/重试、磁盘写入失败、未知版本与 v5 及更早版本
  的可操作错误。

### T4 infra：segment resolver、leaf branch 与 fork

- [blocked-by: T2, T3] `load_entries` 组装完整逻辑条目流；`load_leaf_branch`
  先读 manifest/active，再按 `parentId` 与 leaf 索引按需回读 cold segment。
- 保持现有 `build_context_entries`、compaction cut 与 session-wide bash 配对
  语义；resume 热路径不得无条件解析全部 cold segment，完整导出/inspect 仍显式走
  cold path。
- fork 遇到 cold parent 时按需读父路径，将选中路径复制并重新链接到新的 v7
  child active 段，不保存跨 session segment 引用；保留 `At`/`Before` 和 deferred
  assistant flush 语义。
- 单测覆盖 cold parent lookup、sibling 排除、cold leaf travel、fork copy、
  segment 缺失/损坏诊断。

### T5 infra：compaction seal 的原子提交

- [blocked-by: T4] 为 compaction 成功路径增加「非空被摘要 prefix → 新 cold
  segment + 新 active generation + manifest 原子切换」事务；不覆盖已 sealed 段。
- manifest rename 前后崩溃注入/模拟：旧 manifest 可完整恢复；新 manifest 只引用
  已 fsync 的最终段；未引用 orphan 与临时文件不进入逻辑会话。
- 只在有实际可摘要历史时 seal；manual force 的 empty/already-compacted 错误
  不产生空 cold segment。
- 单测验证重复 compaction 的 cold 段数量、active tail、逻辑 entry 顺序与 cold
  segment 不可变。

### T6 agent：compaction policy 指纹接线

- [blocked-by: T1, T5] 从实际 compaction settings、context window 与
  estimator 版本构造 policy 快照，随新 `CompactionEntry` 写入；threshold、overflow、
  manual 三路径不改变原有摘要/切点语义。
- `clone_entry_with_ids`、导出、context projection 与所有构造 fixture 同步
  policy 字段；legacy/unknown 在诊断中可区分但不改变历史摘要。
- 单测断言四个 policy 字段、迁移条目 unknown、重复 compaction 的快照分别反映
  当次配置。

### T7 consumer：resume、export、inspect 与 observability 适配

- [blocked-by: T3, T4, T5] 迁移 resume/list、session stats、export/import、
  inspect 与 obs 路径发现对单文件假设：路径锚定 manifest/session directory，逻辑
  export 仍输出 JSONL 条目而非 manifest。
- 调整 compaction/session 观测字段与诊断，使 active/cold、migration failure、
  policy unknown 等信息可定位；不新增第二套 session identity 或 wire 方法。
- 复用既有 app/infra 回归测试，覆盖 resume、fork、HTML/JSONL export 和观测路径
  在 v7 目录下的行为。

### T8 specs + BDD：把验收接缝落地

- [blocked-by: T5, T6, T7] 在绑定的 feature 分支更新
  `agent-session-store.feature`：v7 manifest/layout、原子恢复、cold 不可变、
  v6 migration、cold parent lookup 与 fork copy 的 `@req`/`@executable` 场景。
- 更新 `domain-compaction.feature`：compaction seal 保留 active tail，并新增
  policy/窗口指纹规则及 `seal-keeps-active-tail`、`entry-policy-fingerprint`
  等可执行场景。
- 在既有 `bindings_*` 注册新场景；在既有 steps/fixtures 中增加最小步骤，禁止
  新建平行 SessionStore 或 compaction runner seam。
- 运行 rstest-bdd，确认每个 feature 场景都映射到已有 harness。

### T9 门禁与收益复核

- [blocked-by: T8] `just fmt`、相关 `just lint`/clippy、主 crate 相关
  session/compaction 单测与 `cargo test --lib --all-features tests::bdd::` 全绿。
- 运行 `llman sdd validate c2800-update-session-compact-seal-manifest
  --strict --no-interactive`，修复 feature、staleness 与 BDD runner 报告。
- 用现有 `lab_`/回放入口对比 c25 后基线：resume 冷段读取范围、重复 compaction
  次数、磁盘布局增长；把证据与剩余风险写入 verify 报告，不扩展为新的长期格式。
