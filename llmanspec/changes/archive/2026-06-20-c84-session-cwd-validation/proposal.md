---
depends_on: []
---

# c84-session-cwd-validation: 会话工作目录存在性验证

## Why
pi 的 `session-cwd.ts`（59 LOC）在恢复会话时验证存储的 `cwd` 目录是否存在。如果目录已被删除（例如临时工作区被清理），系统会提示用户确认 fallback 到当前目录，而不是静默失败或抛出未处理的异常。

xylitol 的 `SessionManager` 恢复会话时直接使用存储的 cwd，不验证存在性，导致在已删除目录上恢复会话时工具执行静默失败。

## What Changes
- 新增 `src/infra/session/cwd.rs`：
  - `SessionCwdIssue { session_file, session_cwd, fallback_cwd }`
  - `get_missing_session_cwd_issue(sm, fallback_cwd) -> Option<SessionCwdIssue>`
  - `assert_session_cwd_exists(sm, fallback_cwd) -> Result<(), MissingSessionCwdError>`
  - `MissingSessionCwdError` — 含 `SessionCwdIssue` 的自定义错误
  - `format_missing_session_cwd_error(issue)` / `format_missing_session_cwd_prompt(issue)`
- 集成到 CLI startup 路径（`interface/cli/mod.rs`）：恢复会话前调用 `assert_session_cwd_exists`
- 集成到 RPC mode startup：检查并返回错误事件

## Capabilities
- session-persistence

## Impact
- 非破坏性：新增模块 + 可选校验。现有代码不受影响。
- 无新依赖。
