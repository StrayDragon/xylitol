# Tasks — Session 冷段按需索引与旧格式边界

## 测试 seam

- 复用 `agent-session-store` 的既有 BDD feature、bindings 与
  `SessionManager` fixture。
- 复用并扩展 `src/infra/session/manager/tests/v7_layout.rs`、
  `src/infra/session/manager/tests/interrupted_bash.rs` 与相关 deferred/session
  单测。
- 不新建平行 SessionStore、sidecar runner 或独立迁移 harness。

## 实施任务

- [ ] T1 [blocked-by: none] 在绑定分支更新 `agent-session-store` live feature：
  明确 sealed sidecar 的按需恢复与旧 v6/v5 文件不自动迁移、保留并返回可操作错误；
  增加对应 executable 场景并保持 `s24`、`s25` 的原子提交与 fallback 语义。
- [ ] T2 [blocked-by: T1] 扩展 protocol manifest 词汇，增加可选 camelCase
  `indexPath`；在 infra 实现 sidecar 的 `entryIds` / `doneBashIds` 生成、解析、
  session-root path 校验和旧 index/orphan 清理。
- [ ] T3 [blocked-by: T2] 接入 compaction seal：cold JSONL、sidecar、new active
  依次原子写入并同步，manifest 最后提交；任何失败清理本轮 orphan，旧 manifest
  仍可恢复。
- [ ] T4 [blocked-by: T2] 改造 leaf resolver 与 `XySessionStore` done-bash 查询：
  优先用 sidecar 筛选 cold 段，缺失/损坏/未命中时回退扫描；resume/context 不再
  为普通 done 配对无条件解析全部 cold。
- [ ] T5 [blocked-by: T1] 移除 SessionManager 的 v6 storage migration、headerless
  repair 与 cleanup retry；旧 legacy 文件在自动恢复/写入入口返回明确错误并保留，
  用户显式 delete 仍可删除；显式 import parser 不变。
- [ ] T6 [blocked-by: T3, T4, T5] 在既有 infra seam 补 sidecar 写入、manifest
  `indexPath`、旧 manifest fallback、损坏 index fallback、cold branch 命中、
  done-bash 配对和 legacy boundary 回归测试。
- [ ] T7 [blocked-by: T6] 运行 `just fmt`、相关 `just lint`、session/BDD 测试、
  `llman sdd validate` 与 `just qa`；既有全仓 `@req` 结构错误和 tufa live-provider
  不可用状态作为外部/基线阻断记录，不扩大本 change。
