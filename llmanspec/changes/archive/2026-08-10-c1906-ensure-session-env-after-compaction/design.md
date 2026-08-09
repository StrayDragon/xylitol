# Design: c1906 ensure session_env after compaction

> 上游：[`c1905` archive design](../2026-08-10-c1905-update-system-prompt-stable-volatile-split/design.md) D8。
> 分工：本 change = **`session_env` bootstrap**；全栏堆积 = [`c1897`](../../c1897-update-compaction-status-bar-messages/proposal.md)。

## 不变量

进入 provider 的 `history`（含 overflow compact 后的 reload）在继续生成前：

1. 若无 `session_env`，或最新一条的 **date/cwd** 与当前进程快照不一致 → **追加**一条校正 `session_env`（append-only）；
2. 复用 `should_append_session_env` / `snapshot_for_cwd` / `last_session_env`（`agent/prompt/session_env.rs`）；
3. **不**把 cwd/date 写回 system；**不**删除 transcript 中已被 cut 掉的旧 env 行（它们本就不在上下文里）。

## 推荐缝

| 缝 | 动作 |
|---|---|
| ReAct 用户落盘前 | 调用共享 `ensure_session_env_in_history`；若追加则 **persist** |
| overflow reload 后 | **必须** ensure；若追加则 **persist**（同轮重试可见且写盘） |
| `compact_session` 成功后 | 对 leaf `build_context_entries` 视图 ensure；若追加则 **persist**（减空窗） |

## 已钉（0.3）

- **组装时 ensure + 追加则 persist**（三缝一致）；不在仅内存里偷偷补一条却不写 transcript。
- cwd 来源：ReAct/overflow 用 FrozenRoot / 进程 cwd；`compact_session` 用 session header cwd（缺省 `"."`）。

## 非目标

- system 含 pwd
- `AgentStatusBar` keep-latest / drop-all（c1897）
- 改变 `SessionScope::All` 语义

## 测试

- cut 掉早期 env → ensure → 有一条且 cwd/date 正确
- 多条历史 → 取最新比较；cwd 变则再追加
- overflow reload 路径无 env → ensure 后有
