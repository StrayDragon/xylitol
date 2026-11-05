---
depends_on: []
title: "Harden trust store"
branch: main
base_branch: origin/main
base_sha: 78d077258664b34c5b9147336d76994cb6f448b7
---

## Why

`trust.json` 由 `fs::write` 写入，未显式设权限；默认 umask 下通常 0644（实盘当前 0644）。同用户 agent 或其他进程可以直接改写并自信任 workspace，绕过 trust 门禁。

## What Changes

- trust store 0600 原子写；lock 文件 0600；schema version 与旧 0644 文件迁移；写入前校验。
- HMAC key 放 agent dir（`~/.xylitol/trust.key`，0600）；trust.json 带 HMAC；篡改检测。
- trust 决策只允许人类操作：TUI 交互确认；host unary 需要 client capability / 确认；不得暴露为 LLM 可调用工具。
- resume 时重新校验 workspace set 的 trust；不得盲目信任 session 存储的 set。
- additional workspace 不在本 change 范围；先不做。
- 完整性：HMAC key 在 agent dir，同用户 agent 可读；只提高门槛，不能对抗同用户任意代码执行。
- 文档：trust store 是 advisory，不是安全边界；不可信 workspace 用 OS sandbox / 独立用户 / 容器。
- 测试：权限、原子写、tamper、审计、resume 重校验。

## 审计日志

- 落点：`~/.xylitol/audit/trust.log`，0600；JSONL，append-only。
- 事件：`trust_grant` / `trust_deny` / `trust_revoke` / `trust_tamper` / `trust_recheck` / `trust_store_write`。
- 字段：`ts`（RFC3339）、`event`、`actor`（tui / host / cli / system）、`session_id`、`workspace`（canonical path）、`decision`、`source`（tui_prompt / host_unary / resume / startup）、`hmac_ok`、`reason`。
- 轮转：10MB × 5；保留 90 天；可配置；不记录 secrets。
- 用途：用户 review、排查异常 trust 变更、取证；不是安全边界（同用户 agent 可改/删日志）。
- 默认开启；配置键 `trust.audit`（默认 true，`false` 可关）；需落 `runtime-config` specs。

## Capabilities

- `domain-security`：trust store 与审计。
- `server-core`：host trust 决策。
- `app-tui-trust`：人类确认。
- `cli-*`：trust 文件权限与迁移。

## Impact

trust、host、TUI、workspace set、session resume、安全文档。

## Further Notes

- 当前实现：`crates/xylitol-runtime/src/trust/store.rs` 的 `write_file` 用 `fs::write` + rename；无显式 0600。
- 同用户 agent 可通过 bash 工具改写 trust.json，也可读 trust.key；文件权限、HMAC、审计日志不能阻止同用户进程。
- additional workspace 先不做；本 change 只覆盖 primary workspace trust。
- 与 `c3050-add-host-workspace-binding` 协同。

## Open Questions

- HMAC key 在 agent dir 的实际收益；是否需要 keychain 作为可选增强。
- 审计日志保留周期与轮转是否合适。
