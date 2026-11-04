---
depends_on: []
title: "Harden trust store"
---

## Why

`trust.json` 由 `fs::write` 写入，默认权限（当前实际 0644），同用户 agent 或其他进程可以直接改写并自信任 workspace，绕过 trust 门禁。workspace set 的 additional workspace 也依赖 trust 判断，存在安全后门。

## What Changes

- trust store 0600 原子写；lock 文件 0600；schema version；写入前校验。
- trust 决策只允许人类操作：TUI 交互确认；host unary 需要 client capability / 确认；不得暴露为 LLM 可调用工具。
- resume 时重新校验 workspace set 的 trust；不得盲目信任 session 存储的 set。
- additional workspace 默认 ephemeral；持久化必须人类确认并审计。
- 审计：记录 who / when / which workspace / decision；日志脱敏。
- 完整性：HMAC / keychain 签名提高门槛；明确不能对抗同用户任意代码执行。
- 文档：trust store 是 advisory，不是安全边界；不可信 workspace 用 OS sandbox / 独立用户 / 容器。
- 测试：权限、原子写、tamper、审计、resume 重校验。

## Capabilities

- `domain-security`：trust store 与审计。
- `server-core`：host trust 决策。
- `app-tui-trust`：人类确认。
- `cli-*`：trust 文件权限与迁移。

## Impact

trust、host、TUI、workspace set、session resume、安全文档。

## Further Notes

- 当前实现：`crates/xylitol-runtime/src/trust/store.rs` 的 `write_file` 用 `fs::write` + rename；无显式 0600。
- 同用户 agent 可通过 bash 工具改写 trust.json；文件权限不能阻止同用户进程。真正的隔离需要 OS sandbox / 独立用户。
- 与 `c3050-add-host-workspace-binding` 协同：additional workspace 的 trust 与审计。

## Open Questions

- HMAC key 放 OS keychain 还是 agent dir；同用户可读 keychain 时的实际收益。
- 审计日志落点与保留周期。
- 是否要求每次 additional workspace 都重新确认，还是 session 内记住。
