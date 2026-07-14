# Design — c995

## Cancel vs observe

| 事件 | 语义 |
|------|------|
| `session_before_tree` / `session_before_switch` | Blocked → 操作中止，错误可观测 |
| `session_tree` / `session_shutdown` | observe fail-open |

## shutdown

- MUST：因 `switch_session`（及显式 new 替换）离开当前会话时发出
- MAY：进程退出 — 待库级 teardown 缝出现再升 MUST

## BDD 操作

- `打开会话树` / `切换会话 target`（字典扩 c990）
