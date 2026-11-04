---
depends_on: []
title: "Harden trust store"
---

## Why

`trust.json` 由 `fs::write` 写入，默认权限（当前实际 0644），同用户 agent 或其他进程可以直接改写并自信任 workspace，绕过 trust 门禁。workspace set 的 additional workspace 也依赖 trust 判断，存在安全后门。

## What Changes

- trust store 0600 原子写；lock 文件 0600；schema version；写入前校验。
- HMAC key 放 agent dir（`~/.xylitol/trust.key`，0600）；trust.json 带 HMAC；篡改检测。
- 审计日志放 agent dir（`~/.xylitol/audit/trust.log`，0600）；记录 who / when / which workspace / decision。
- trust 决策只允许人类操作：TUI 交互确认；host unary 需要 client capability / 确认；不得暴露为 LLM 可调用工具。
- resume 时重新校验 workspace set 的 trust；不得盲目信任 session 存储的 set。
- additional workspace 重新确认机制：设计保留，**暂不实现**；agent 有大量工具可绕过文件确认，实际收益有限；后续讨论。
- 完整性：HMAC key 在 agent dir，同用户 agent 可读；只提高门槛，不能对抗同用户任意代码执行。
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
- 同用户 agent 可通过 bash 工具改写 trust.json，也可读 trust.key；文件权限与 HMAC 不能阻止同用户进程。
- additional workspace 重新确认暂不做；记录为后续讨论。
- 与 `c3050-add-host-workspace-binding` 协同：additional workspace 的 trust 与审计。

## Open Questions

- HMAC key 在 agent dir 的实际收益；是否需要 keychain 作为可选增强。
- 审计日志保留周期与轮转。
- additional workspace 重新确认是否值得做；agent 工具绕过如何缓解。
