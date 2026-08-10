# Design: c1906 ensure session_env after compaction

> 上游：[`c1905/design.md`](../c1905-update-system-prompt-stable-volatile-split/design.md) D8。
> 分工：本 change = **`session_env` bootstrap**；全栏堆积 = [`c1897`](../c1897-update-compaction-status-bar-messages/proposal.md)。

## 不变量

进入 provider 的 `history`（含 overflow compact 后的 reload）在继续生成前：

1. 若无 `session_env`，或最新一条的 **date/cwd** 与当前进程快照不一致 → **追加**一条校正 `session_env`（append-only）；
2. 复用 `should_append_session_env` / `snapshot_for_cwd` / `last_session_env`（`agent/prompt/session_env.rs`）；
3. **不**把 cwd/date 写回 system；**不**删除 transcript 中已被 cut 掉的旧 env 行（它们本就不在上下文里）。

## 推荐缝

| 缝 | 动作 |
|---|---|
| ReAct 用户落盘前 | 已有注入 → 改为调用共享 `ensure_session_env_*` |
| overflow reload 后 | **必须** ensure（内存；persist 可选但建议） |
| `compact_session` 成功后 | 若 leaf 上下文无有效 env → persist 一条（减空窗） |

## 非目标

- system 含 pwd
- `AgentStatusBar` keep-latest / drop-all（c1897）
- 改变 `SessionScope::All` 语义

## 测试

- cut 掉早期 env → ensure → 有一条且 cwd/date 正确
- 多条历史 → 取最新比较；cwd 变则再追加
- overflow reload 路径无 env → ensure 后有
