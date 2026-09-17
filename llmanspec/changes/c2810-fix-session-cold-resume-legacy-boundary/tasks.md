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
  明确 sealed sidecar 的按需恢复、list 跳过不可解析条目与旧 v6/v5 文件不自动迁移、
  保留并返回可操作错误；增加对应 executable 场景并保持 `s24`、`s25` 的原子提交与
  fallback 语义；同步收紧 `s21`（list 跳过，不再「识别或迁移 v6」）与 `s25`
  （恢复只走可解析 v7 的 latest leaf）。
- [ ] T2 [blocked-by: T1] 扩展 protocol manifest 词汇，增加可选 camelCase
  `indexPath`；在 infra 实现 sidecar 的 `entryIds` / `doneBashIds` 生成、解析、
  session-root path 校验和旧 index/orphan 清理。
- [ ] T3 [blocked-by: T2] 接入 compaction seal：cold JSONL、sidecar、new active
  依次原子写入并同步，manifest 最后提交；任何失败清理本轮 orphan，旧 manifest
  仍可恢复。
- [ ] T4 [blocked-by: T2] 改造 leaf resolver 与 `XySessionStore` done-bash 查询：
  优先用 sidecar 筛选 cold 段，缺失/损坏/未命中时回退扫描；resume/context 不再
  为普通 done 配对无条件解析全部 cold。**MUST NOT 破坏 `build_session_context` 的
  LLM API 信息投影**（`thinkingLevel` / `model` 来自 leaf 分支 `modelChange` /
  `thinkingLevelChange`）——改接 `load_done_bash_ids` 后投影语义保持不变。
- [ ] T5 [blocked-by: T1] 零兼容移除 SessionManager 的 v6 storage migration、
  headerless repair 与 cleanup retry；旧 legacy 文件在自动恢复/写入入口返回明确
  错误并保留，`list_sessions` 直接跳过 legacy（不迁移、不解析展示），用户显式
  delete 仍可删除；显式 import parser 不变。
- [ ] T6 [blocked-by: T3, T4, T5] 在既有 infra seam 补 sidecar 写入、manifest
  `indexPath`、旧 manifest fallback、损坏 index fallback、cold branch 命中、
  done-bash 配对和 legacy boundary 回归测试；**把既有迁移成功断言测试（如
  `v6_file_migrates_once` 等）改为 zero-compat 断言**（返回不支持错误 / 文件保留 /
  list 跳过 / delete 可清理），不保留迁移 harness。
- [ ] T7 [blocked-by: T6] 运行 `just fmt`、相关 `just lint`、session/BDD 测试、
  `llman-sdd validate` 与 `just qa`；既有全仓 `@req` 结构错误和 tufa live-provider
  不可用状态作为外部/基线阻断记录，不扩大本 change。
