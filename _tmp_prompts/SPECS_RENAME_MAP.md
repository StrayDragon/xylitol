# Specs 命名迁移映射表

> 根据 `llmanspec/config.yaml rules.proposal` 层前缀规则整理。
> **先提案，等主线确认后再搬目录。**
> 生成时间：2026-07-10

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

## 需迁移（按建议优先级分组）

### P0 — 明确层归属，语义清晰

| 现名 | 建议名 | 理由 |
|---|---|---|
| `app-protocol` | `protocol-app` | 属 protocol 层（client↔core 线协议）；现名带 `app-` 易与产品 app 层混淆 |
| `bash-execution` | `infra-bash` | bash 工具由 infra 层实现和执行 |
| `clipboard` | `infra-clipboard` | 剪贴板操作是 infra 层能力 |
| `git-utils` | `infra-git` | git 集成属 infra 层 |
| `image-utils` | `infra-image` | 图片工具属 infra 层 |
| `network-config` | `infra-network` | 网络配置属 infra 层 |
| `process-mgmt` | `infra-process` | 进程管理属 infra 层 |
| `provider-adapter` | `infra-provider` | provider adapter 属 infra 层 |
| `hook-system` | `agent-hooks` | hooks 是 agent 循环的扩展机制 |
| `prompt-template` | `agent-prompt` | prompt 模板属 agent 层 |
| `tool-system` | `agent-tools` | 工具系统属 agent 层 |
| `session-persistence` | `agent-session-store` | 会话持久化是 agent session 的一部分（已有 `agent-session`）|
| `print-output` | `cli-print` | 纯文本输出是 CLI 模式的呈现方式 |
| `compaction` | `domain-compaction` | 压缩（对话历史压缩）是 domain 层概念 |
| `security-policy` | `domain-security` | 安全策略是 domain 层概念 |

### P1 — 跨层 / 测试 / 基础设施（需更多讨论）

| 现名 | 建议名 | 理由与疑问 |
|---|---|---|
| `architecture` | `meta-architecture` | 跨层架构说明，不属单层。也可 `meta-architecture` 或 `docs-architecture` |
| `layer-architecture` | `meta-layer-arch` | 同上，与 `architecture` 合并或明确分工 |
| `bdd-tests` | `test-bdd` | 属测试基础设施；`test-*` 前缀 |
| `testing-standards` | `test-standards` | 属测试规范；`test-*` 前缀 |
| `test-infra` | `test-infra` | **已合规则**（test- 前缀），保留 |
| `fake-provider` | `test-fake-provider` | 假 provider 是测试替身，归测试域 |
| `provider-integration` | `test-provider-integration` | provider 集成测试规范 |
| `model-registry` | `runtime-model-registry` | model registry 是 runtime_protocol 层 port |
| `resource-discovery` | `runtime-resource-discovery` | 资源发现是 runtime_protocol 层能力 |
| `diagnostics` | `infra-diagnostics` | 诊断/错误报告是 infra 层能力 |
| `debug-log` | `infra-logging` | 日志是 infra 层能力 |
| `workspace-structure` | `meta-workspace` | 跨层工作区结构说明 |
| `user-experience` | `meta-ux` | 跨层 UX 准则 |
| `build-config` | `meta-build` | 构建配置跨层 |
| `diff-review` | ~~（已删除）~~ | 已在本批清理中删除 |

## 操作建议

1. **P0 优先级最高** — 层归属清晰，可安全执行 `git mv`。约 15 个目录。
2. **P1 需要更多讨论** — 特别是跨层/测试命名（`architecture` vs `layer-architecture` 合并、`model-registry` 是否真属 `runtime-*`）。
3. **合并候选**：`architecture` + `layer-architecture`；`bdd-tests` + `testing-standards` + `test-infra`。
4. **建议策略**：每批 5-10 个目录，分批 `git mv`，每批后 `llman sdd validate --all --strict` 确认不炸。
5. **暂不动的**：`app-tui`（等 c450 归档）；`diff-review`（已删）。

> 确认后删除本文件，按批次执行 `git mv`。
