# language: zh-CN
# capability: infra-observability
# purpose: Debug 观察与 provider trace 管线：env 驱动激活的 fastrace 时间线 + log 文件 + provider-trace.jsonl（~/.xylitol/logs/），file-only 不毁 TUI 渲染，在组合根 app::cli::run 统一装配；禁止 tracing 双栈；低频跨层 span 可关联；导出名与父子树对齐 infra-otel 产品词汇。
# scope: src/infra/observability/, src/app/cli/

功能: infra-observability

  @req:r1461
  规则: debug-observation-pipeline
    应用 MUST 在组合根恰好一次装配观测后端：级别 log 与本地 fastrace FileReporter MUST 只写入 agent 日志目录下的文件（如 xylitol.log / provider-trace.jsonl），MUST NEVER 写 stdout 或 stderr，以免破坏 TUI Inline viewport 与 DSR。禁止使用会打印到 stderr 的 ConsoleReporter。本地 file 激活 MUST 仅由环境与构建配置驱动（无 CLI flag）：debug 构建（cfg(debug_assertions)）MUST 默认启用；release MUST 默认关闭，经 XYLITOL_DEBUG / RUST_LOG（级别日志）与 XYLITOL_PROVIDER_TRACE（provider timeline）显式打开。可选远程 OTLP 出口 MAY 由 AppConfig.[otel] 控制且默认关闭（见 infra-otel），MUST NOT 与本地 file 闸混为一谈。文件写入 MUST 同步 append（兼容 panic=abort），unix 下文件模式 MUST 为 0o600。TUI 层 MUST 只调用 log 宏或 fastrace API，MUST NOT 自行安装 Reporter。时间线栈 MUST 为 fastrace；MUST NOT 再依赖 tracing / tracing-subscriber。
    # verified-by: src/infra/observability/file_reporter.rs
  @req:r1462
  规则: provider-raw-mapped-对照
    当 provider tracing 激活时，每次 provider HTTP 流 MUST 在同一 fastrace 请求 span / request_id 下记录原始协议事件与映射后的 XyChunk 变体（Event），使 agent 能区分上游通道混写与适配器映射错误；记录 MUST 只经 file-only Reporter 写入 agent 日志目录专用文件（禁止 stdout/stderr 与 ConsoleReporter），MUST NOT 包含 Authorization 或 API-key 头值，且 MUST NOT 经 script hook 分发。

    场景: provider-raw-mapped-pairing-headless
      假如 mock 上游按正常完成脚本回放 Anthropic SSE
      当 以 io=truncated 的观测闸对 mock 上游驱动一次流式生成并耗尽全部事件
      那么 同一 llm.request span 上同时记录 raw 与 mapped 事件

  @req:r1463
  规则: provider-trace-闸门
    Provider tracing 在 cfg(debug_assertions) 下 MUST 默认安装 Reporter，在 release 构建 MUST 默认不安装；release MUST 经 XYLITOL_PROVIDER_TRACE 显式打开；tracing 未激活时 MUST 避免对 SSE 载荷做昂贵序列化；新增跨层低频 span MUST 遵守同一闸门（关闸零/近零开销）。

    场景: trace-gate-off-emits-nothing
      假如 mock 模型先 tool 后无 tool
      当 以未开启观测闸运行一次带工具调用的 agent 回合
      那么 默认观测闸为关且回合正常完成不出口任何 span
  @req:r1464
  规则: fastrace-单栈
    仓库观测时间线 MUST 仅使用 fastrace；级别诊断 MUST 使用 log 门面；Cargo 与 src MUST NOT 依赖 tracing 或 tracing-subscriber；MUST NOT 引入 fastrace-tracing 兼容层。允许 fastrace-futures 等官方 companion（非 tracing 桥）。
    # verified-by: Cargo.toml
  @req:r1465
  规则: agent-low-freq-spans
    当 provider tracing 激活时，一次用户触发的 agent 处理 MUST 提供可关联的低频 fastrace span：根 agent.turn；每步 agent.iteration；模型流 llm.request（可与 request_id 对照）；可选 tool.execute；若发生会话 compaction 则 MUST 另有 agent.compaction（挂于该 turn 下，或无 turn 时为独立根）。主路径子 span MUST 挂在 turn/iteration/compaction 父节点之下（共享 trace_id），MUST NOT 在活跃 turn 下各自无关 random 根。agent.turn 结束时 MUST 能区分正常完成与用户 abort（见 infra-otel otel20）。MUST NOT 为每条 SSE / 每帧 TUI tick 安装无采样 span。默认 MUST NOT 启用 OpenTelemetry/Jaeger 等外部导出 Reporter；远程 OTLP 仅可经独立 [otel] 配置 + feature otel 显式 opt-in（见 infra-otel），且不得写 stdout/stderr。
    # verified-by: llmanspec/specs/infra-otel/infra-otel.feature
