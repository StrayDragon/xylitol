# xylitol 业务状态机梳理

围绕 `llmanspec/specs/` 与对应 Rust 实现，把每个业务模块写成可运行的 xstate 状态机。
每个模块同时给出：TypeScript 状态机（v5 语法、v6 就绪写法）、mermaid 状态图、bun 可执行验证场景。

## 约定

| 项 | 内容 |
|---|---|
| 入口 | `setup({ types, guards, actions }).createMachine({...})`，v5 与 v6 都接受 |
| 禁止 | v4 遗留：`strict` / `schema` / `services` / `cond` / `predictableActionArguments` |
| guard 求值 | 按动作执行前的 context 求值，需要预判的字段在 guard 内自行推算 |
| 保留名 | 状态名避开 xstate v5 保留名（如 `held`），一律用语义名 |
| 场景 | 每组用 `runScenarioGroups` 新建 actor，组间不串状态 |
| 锚点 | 每文件 JSDoc 写 `@spec llmanspec/specs/<cap>/<cap>.feature#@req:<id>` |

## 校验

```bash
cd llmanspec/bloc
bun install
bun run scripts/verify.ts          # 全量跑通 + 锚点核对
bun run scripts/gen-diagrams.ts    # 重新生成 mermaid 与索引
bunx tsc --noEmit                  # 类型检查
```

## 覆盖概览

共 41 个状态机，6 个子包，引用 247 个 spec 文件、247 个 @req 锚点。

### agent-domain（7 个）

| 模块 | 职责 | 锚点数 | 代码 | 图 |
|---|---|---|---|---|
| `agent-todo` | 待办清单 SSOT：latest-wins 持久、三态、多 in_progress、只读投影 | 7 | [agent-todo.ts](agent-domain/agent-todo.ts) | [agent-todo.md](agent-domain/agent-todo.md) |
| `codemode-runtime` | run_code 执行：暴露交集、声明预算、沙箱限额、嵌套 dispatch | 8 | [codemode-runtime.ts](agent-domain/codemode-runtime.ts) | [codemode-runtime.md](agent-domain/codemode-runtime.md) |
| `compaction` | 上下文压缩触发与摘要：阈值、force、overflow、防抖、双摘要 | 7 | [compaction.ts](agent-domain/compaction.ts) | [compaction.md](agent-domain/compaction.md) |
| `goal-lifecycle` | 会话级目标：合法转移表、evaluator 判定、逐 turn 续跑 | 5 | [goal-lifecycle.ts](agent-domain/goal-lifecycle.ts) | [goal-lifecycle.md](agent-domain/goal-lifecycle.md) |
| `hook-dispatch` | hook 匹配与脚本动作：allow/block/modify、失败 fail-closed | 6 | [hook-dispatch.ts](agent-domain/hook-dispatch.ts) | [hook-dispatch.md](agent-domain/hook-dispatch.md) |
| `readonly-subagent` | 只读子代理：零装配、隔离 runtime、只读工具集、父级取消传播 | 4 | [readonly-subagent.ts](agent-domain/readonly-subagent.ts) | [readonly-subagent.md](agent-domain/readonly-subagent.md) |
| `workflow-engine` | 任务图执行：七阶段容器、任务转移表、批准一次消费 | 3 | [workflow-engine.ts](agent-domain/workflow-engine.ts) | [workflow-engine.md](agent-domain/workflow-engine.md) |

### app-surface（6 个）

| 模块 | 职责 | 锚点数 | 代码 | 图 |
|---|---|---|---|---|
| `ask-tool` | TUI 问卷：仅 TUI 装配、槽挂载、跳过成功、载荷回写 | 8 | [ask-tool.ts](app-surface/ask-tool.ts) | [ask-tool.md](app-surface/ask-tool.md) |
| `footer-composition` | footer 单行字段序：used 来源标注、占用比、cache 三态、截断 | 6 | [footer-composition.ts](app-surface/footer-composition.ts) | [footer-composition.md](app-surface/footer-composition.md) |
| `print-rendering` | 非交互输出：stdout 流式、reasoning 走 stderr、去重、非零退出 | 7 | [print-rendering.ts](app-surface/print-rendering.ts) | [print-rendering.md](app-surface/print-rendering.md) |
| `session-tree-slot` | 会话树槽：打开、travel、过滤、折叠、fork、标签持久 | 8 | [session-tree-slot.ts](app-surface/session-tree-slot.ts) | [session-tree-slot.md](app-surface/session-tree-slot.md) |
| `tui-host-lifecycle` | TUI 宿主：attach、raw 终端、尺寸降级、重载、退出恢复 | 7 | [tui-host-lifecycle.ts](app-surface/tui-host-lifecycle.ts) | [tui-host-lifecycle.md](app-surface/tui-host-lifecycle.md) |
| `tui-input-routing` | 键位语义：Esc/Ctrl+C 取消与退出、steer/follow-up、双 Esc 会话树 | 5 | [tui-input-routing.ts](app-surface/tui-input-routing.ts) | [tui-input-routing.md](app-surface/tui-input-routing.md) |

### config-meta（7 个）

| 模块 | 职责 | 锚点数 | 代码 | 图 |
|---|---|---|---|---|
| `boot-assembly` | 启动装配：argv → profile → Kernel::boot、表面与 ops 区分、no-silent-fallback | 12 | [boot-assembly.ts](config-meta/boot-assembly.ts) | [boot-assembly.md](config-meta/boot-assembly.md) |
| `config-load-merge` | 配置分层加载与深合并：三层链、conf.d 片段、两阶段未知键校验、fail-closed | 12 | [config-load-merge.ts](config-meta/config-load-merge.ts) | [config-load-merge.md](config-meta/config-load-merge.md) |
| `fff-index-lifecycle` | fff 检索：惰性建索引、回落内置、全模式可取消 | 5 | [fff-index-lifecycle.ts](config-meta/fff-index-lifecycle.ts) | [fff-index-lifecycle.md](config-meta/fff-index-lifecycle.md) |
| `model-resolution` | 模型解析与回退：显式条目注册、档位 clamp、任务级回退、reload 重建 | 11 | [model-resolution.ts](config-meta/model-resolution.ts) | [model-resolution.md](config-meta/model-resolution.md) |
| `otel-export` | OTEL 出口：默认不收集、opt-in、失败不阻塞、与文件管线 fanout | 8 | [otel-export.ts](config-meta/otel-export.ts) | [otel-export.md](config-meta/otel-export.md) |
| `plugin-assembly` | 外进程插件装配与启停：批准持久化、config 停用最高优先、mcp 声明与命令委派 | 11 | [plugin-assembly.ts](config-meta/plugin-assembly.ts) | [plugin-assembly.md](config-meta/plugin-assembly.md) |
| `resource-discovery` | 统一资源发现与热重载：skills/themes/context、Trust 门禁、reload 报告 | 6 | [resource-discovery.ts](config-meta/resource-discovery.ts) | [resource-discovery.md](config-meta/resource-discovery.md) |

### core-runtime（9 个）

| 模块 | 职责 | 锚点数 | 代码 | 图 |
|---|---|---|---|---|
| `abort` | 中止语义：取消流、清 steer、留 follow_up、跳过 LLM 结算 | 5 | [abort.ts](core-runtime/abort.ts) | [abort.md](core-runtime/abort.md) |
| `auto-retry` | provider 失败重试：退避、状态事件、耗尽与中止 | 4 | [auto-retry.ts](core-runtime/auto-retry.ts) | [auto-retry.md](core-runtime/auto-retry.md) |
| `pending-queues` | steer 与 follow_up 双队列：入队、drain、逐项撤销、逐条成 turn | 5 | [pending-queues.ts](core-runtime/pending-queues.ts) | [pending-queues.md](core-runtime/pending-queues.md) |
| `react-loop` | ReAct 内环：模型调用、流式意图、工具批续跑、停步收尾 | 6 | [react-loop.ts](core-runtime/react-loop.ts) | [react-loop.md](core-runtime/react-loop.md) |
| `session-mailbox` | 有界命令信箱单写者：排队、背压、逐条消费 | 3 | [session-mailbox.ts](core-runtime/session-mailbox.ts) | [session-mailbox.md](core-runtime/session-mailbox.md) |
| `session-slot-lifecycle` | 会话槽的占用、宽限与驱逐，含序号水位续接 | 4 | [session-slot.ts](core-runtime/session-slot.ts) | [session-slot.md](core-runtime/session-slot.md) |
| `session-store-append` | 会话 JSONL 追加存储：串行写、fsync、跳过告警、CWD 回退 | 8 | [session-store-append.ts](core-runtime/session-store-append.ts) | [session-store-append.md](core-runtime/session-store-append.md) |
| `tool-batch` | 工具批调度：barrier 串行、parallel 并行、源序回写 | 6 | [tool-batch.ts](core-runtime/tool-batch.ts) | [tool-batch.md](core-runtime/tool-batch.md) |
| `turn-coordinator` | 单 turn 相位机：空闲→起始→流式→收尾，含忙拒绝与取消 | 5 | [turn-coordinator.ts](core-runtime/turn-coordinator.ts) | [turn-coordinator.md](core-runtime/turn-coordinator.md) |

### protocol-transport（6 个）

| 模块 | 职责 | 锚点数 | 代码 | 图 |
|---|---|---|---|---|
| `approval-request` | 远端审批/问卷：首答生效、人类限定、槽占用排队与队满即时拒绝 | 3 | [approval-request.ts](protocol-transport/approval-request.ts) | [approval-request.md](protocol-transport/approval-request.md) |
| `journal-replay` | 会话 journal 重放：序号水位、partial 补齐、截断 resync、seq 不变量 | 5 | [journal-replay.ts](protocol-transport/journal-replay.ts) | [journal-replay.md](protocol-transport/journal-replay.md) |
| `jsonrpc-envelope` | JSON-RPC 2.0 文本信封：握手协商、方法名寻址、标准码与业务码拒绝 | 6 | [jsonrpc-envelope.ts](protocol-transport/jsonrpc-envelope.ts) | [jsonrpc-envelope.md](protocol-transport/jsonrpc-envelope.md) |
| `serve-lifecycle` | Serve 进程生命周期：persistent/last-client、空闲宽限、ensure-host 版本决策 | 4 | [serve-lifecycle.ts](protocol-transport/serve-lifecycle.ts) | [serve-lifecycle.md](protocol-transport/serve-lifecycle.md) |
| `serve-readiness` | Serve 启动就绪窗口：starting/ready/stopping/failed、绑定占用禁重试、注册文件自检 | 6 | [serve-readiness.ts](protocol-transport/serve-readiness.ts) | [serve-readiness.md](protocol-transport/serve-readiness.md) |
| `ws-session-channel` | WS /rpc 通道：升级、纯 JSON-RPC 文本、订阅跨回合存活、重连 resync | 7 | [ws-session-channel.ts](protocol-transport/ws-session-channel.ts) | [ws-session-channel.md](protocol-transport/ws-session-channel.md) |

### security-infra（6 个）

| 模块 | 职责 | 锚点数 | 代码 | 图 |
|---|---|---|---|---|
| `bash-execution` | bash 子进程执行：流式输出、取消杀树、超时升级、截断溢出与显式结局 | 7 | [bash-execution.ts](security-infra/bash-execution.ts) | [bash-execution.md](security-infra/bash-execution.md) |
| `lsp-dap-tool-registration` | LSP/DAP 工具装配：零配置零装配、首调用启动、相位显式、崩溃后重建一次 | 6 | [lsp-dap-tool-registration.ts](security-infra/lsp-dap-tool-registration.ts) | [lsp-dap-tool-registration.md](security-infra/lsp-dap-tool-registration.md) |
| `mcp-server-lifecycle` | MCP 服务器装配与连接：零配置零装配、失败诊断、重载、last-known 快照、resume 释放 | 8 | [mcp-server-lifecycle.ts](security-infra/mcp-server-lifecycle.ts) | [mcp-server-lifecycle.md](security-infra/mcp-server-lifecycle.md) |
| `permission-gate` | 工具分发前 permission 裁决：路径、网络域、MCP 允许列表、进程工作区，默认拒绝未列出 | 9 | [permission-gate.ts](security-infra/permission-gate.ts) | [permission-gate.md](security-infra/permission-gate.md) |
| `provider-stream-terminal` | Provider SSE 流终态约束：Done 唯一、坏帧守卫、结构化错误、abort finalize、stop_reason 映射 | 6 | [provider-stream-terminal.ts](security-infra/provider-stream-terminal.ts) | [provider-stream-terminal.md](security-infra/provider-stream-terminal.md) |
| `trust-resolution` | 项目信任按固定优先级解析：覆盖、无输入、store、UI 提示、回退拒绝 | 11 | [trust-resolution.ts](security-infra/trust-resolution.ts) | [trust-resolution.md](security-infra/trust-resolution.md) |
