# Specs 命名迁移映射表 (✅ 已完成)

> 生成时间：2026-07-10
> **所有 P0 + 选定的 P1 已于 2026-07-10 执行完成。**

## 已合规（不动）

| 当前名 | 前缀 | 理由 |
|---|---|---|
| `agent-runtime` | `agent-*` | 符合 agent 层前缀 |
| `agent-session` | `agent-*` | 同上 |
| `app-tui` | `app-tui-*` | 单体待退役；c450 归档前保留跨切面不变量 |
| `app-tui-bridge` | `app-tui-*` | 新产品分裂合乎规则 |
| `app-tui-chrome` | `app-tui-*` | 同上 |
| `app-tui-commands` | `app-tui-*` | 同上 |
| `app-tui-host` | `app-tui-*` | 同上 |
| `app-tui-input` | `app-tui-*` | 同上 |
| `app-tui-transcript` | `app-tui-*` | 同上 |
| `cli-entry` | `cli-*` | 符合 CLI 层前缀 |
| `package-tui-autocomplete` | `package-tui-*` | 符合作业区包前缀 |
| `package-tui-editor` | `package-tui-*` | 同上 |
| `package-tui-editor-autocomplete` | `package-tui-*` | 同上 |
| `package-tui-engine` | `package-tui-*` | 同上 |
| `package-tui-paste-burst` | `package-tui-*` | 同上 |
| `package-tui-terminal-protocol` | `package-tui-*` | 同上 |
| `package-tui-testing` | `package-tui-*` | 同上 |
| `runtime-config` | `runtime-*` | 符合 runtime_protocol 层前缀 |
| `server-reverse-rpc` | `server-*` | 符合 server 层前缀 |
| `server-runtime` | `server-*` | 同上 |
| `server-ws` | `server-*` | 同上 |

## ✅ 已执行（P0 + P1 选定）

### P0（15/15 完成）

| 原名 | 新名 |
|---|---|
| `app-protocol` | ✅ `protocol-app` |
| `bash-execution` | ✅ `infra-bash` |
| `clipboard` | ✅ `infra-clipboard` |
| `git-utils` | ✅ `infra-git` |
| `image-utils` | ✅ `infra-image` |
| `network-config` | ✅ `infra-network` |
| `process-mgmt` | ✅ `infra-process` |
| `provider-adapter` | ✅ `infra-provider` |
| `hook-system` | ✅ `agent-hooks` |
| `prompt-template` | ✅ `agent-prompt` |
| `tool-system` | ✅ `agent-tools` |
| `session-persistence` | ✅ `agent-session-store` |
| `print-output` | ✅ `cli-print` |
| `compaction` | ✅ `domain-compaction` |
| `security-policy` | ✅ `domain-security` |

### P1 选定（8/8 完成）

| 原名 | 新名 |
|---|---|
| `bdd-tests` | ✅ `test-bdd` |
| `testing-standards` | ✅ `test-standards` |
| `fake-provider` | ✅ `test-fake-provider` |
| `provider-integration` | ✅ `test-provider-integration` |
| `model-registry` | ✅ `runtime-model-registry` |
| `resource-discovery` | ✅ `runtime-resource-discovery` |
| `diagnostics` | ✅ `infra-diagnostics` |
| `debug-log` | ✅ `infra-logging` |

### 本批不做

| 原名 | 原因 |
|---|---|
| `architecture` / `layer-architecture` | 需主线再议合并 |
| `workspace-structure` | 需主线再议 |
| `user-experience` | 需主线再议 |
| `build-config` | 需主线再议 |
| `test-infra` | 已合规（`test-*` 前缀） |

> 映射表已完成使命，可删除此文件。
