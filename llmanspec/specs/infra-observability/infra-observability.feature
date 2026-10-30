# language: zh-CN
# purpose: Debug 观察与 provider trace 管线：env 驱动激活的 fastrace 时间线 + log 文件 + provider-trace.jsonl（~/.xylitol/logs/），file-only 不毁 TUI 渲染，在组合根 app::cli::run 统一装配；禁止 tracing 双栈；低频跨层 span 可关联；导出名与父子树对齐 infra-otel 产品词汇。

功能: infra-observability

  @req:r1461
  规则: debug-observation-pipeline
    应用 须在组合根恰好一次装配观测后端：级别 log 与本地 fastrace FileReporter 须只写入 agent 日志目录下的文件（如 xylitol.log / provider-trace.jsonl），须 NEVER 写 stdout 或 stderr，以免破坏 TUI Inline viewport 与 DSR。禁止使用会打印到 stderr 的 ConsoleReporter。本地 file 激活 须仅由环境与构建配置驱动（无 CLI flag）：debug 构建（cfg(debug_assertions)）须默认启用；release 须默认关闭，经 XYLITOL_DEBUG / RUST_LOG（级别日志）与 XYLITOL_PROVIDER_TRACE（provider timeline）显式打开。可选远程 OTLP 出口 MAY 由 AppConfig.[otel] 控制且默认关闭（见 infra-otel），不得与本地 file 闸混为一谈。文件写入 须同步 append（兼容 panic=abort），unix 下文件模式 须为 0o600。TUI 层 须只调用 log 宏或 fastrace API，不得自行安装 Reporter。时间线栈 须为 fastrace；不得再依赖 tracing / tracing-subscriber。

    场景: 组合根单一观测出口
      当 读取观测后端装配
      那么 组合根恰一次装配且落 agent 日志目录
  @req:r1462
  规则: provider-raw-mapped-对照
    当 provider tracing 激活时，每次 provider HTTP 流 须在同一 fastrace 请求 span / request_id 下记录原始协议事件与映射后的 XyChunk 变体（Event），使 agent 能区分上游通道混写与适配器映射错误；记录 须只经 file-only Reporter 写入 agent 日志目录专用文件（禁止 stdout/stderr 与 ConsoleReporter），不得包含 Authorization 或 API-key 头值，且 不得经 script hook 分发。

    场景: 厂商原文映射对拍无头
      假如 mock 上游按正常完成脚本回放 Anthropic SSE
      当 以 io=truncated 的观测闸对 mock 上游驱动一次流式生成并耗尽全部事件
      那么 同一 llm.request span 上同时记录 raw 与 mapped 事件

  @req:r1463
  规则: provider-trace-闸门
    Provider tracing 在 cfg(debug_assertions) 下 须默认安装 Reporter，在 release 构建 须默认不安装；release 须经 XYLITOL_PROVIDER_TRACE 显式打开；tracing 未激活时 须避免对 SSE 载荷做昂贵序列化；新增跨层低频 span 须遵守同一闸门（关闸零/近零开销）。

    场景: 追踪门禁关发出无输出
      假如 mock 模型先 tool 后无 tool
      当 以未开启观测闸运行一次带工具调用的 agent 回合
      那么 默认观测闸为关且回合正常完成不出口任何 span
  @req:r1464
  规则: fastrace-单栈
    仓库观测时间线 须仅使用 fastrace；级别诊断 须使用 log 门面；Cargo 与 src 不得依赖 tracing 或 tracing-subscriber；不得引入 fastrace-tracing 兼容层。允许 fastrace-futures 等官方 companion（非 tracing 桥）。

    场景: fastrace单一栈
      当 读取观测栈依赖清单
      那么 仅用 fastrace 与 log 且无 tracing

  @req:r3006
  规则: rutis-loader-notice-level
    Host 不链接 rutis-bridge 时，rutis-loader 的 `LoaderService` 会产生 `ErrorKind::NotServedByName`。该提示 须降级为 debug 级，不得以 error 级污染 `xylitol::rutis` 日志；其余 rutis 异步错误 仍须保持 error 级。需要排查按名服务时 须经 `RUST_LOG=xylitol::rutis=debug` 显式打开。

    场景: loader 按名提示降为 debug
      假如 Host 未链接 rutis-bridge 且 loader 产生 NotServedByName
      当 写入 xylitol::rutis 日志
      那么 该提示为 debug 级且不产生 error 级记录

  @req:r1465
  规则: agent-low-freq-spans
    当 provider tracing 激活时，一次用户触发的 agent 处理 须提供可关联的低频 fastrace span：根 agent.turn；每步 agent.iteration；模型流 llm.request（可与 request_id 对照）；可选 tool.execute；若发生会话 compaction 则 须另有 agent.compaction（挂于该 turn 下，或无 turn 时为独立根）。主路径子 span 须挂在 turn/iteration/compaction 父节点之下（共享 trace_id），不得在活跃 turn 下各自无关 random 根。agent.turn 结束时 须能区分正常完成与用户 abort（见 infra-otel otel20）。不得为每条 SSE / 每帧 TUI tick 安装无采样 span。默认 不得启用 OpenTelemetry/Jaeger 等外部导出 Reporter；远程 OTLP 仅可经独立 [otel] 配置 + feature otel 显式 opt-in（见 infra-otel），且不得写 stdout/stderr。

    场景: 厂商追踪关联跨度
      当 读取低频观测 span 定义
      那么 agent.turn 与每步 span 可关联

  @req:r3003
  规则: rutis-异步错误出口
    Kernel 装配 rutis 根上下文时 须安装自定义 ErrorSink，把无人 await 的 rutis 错误经 log 门面写入既有 file-only 观测管线（target 固定为 xylitol::rutis）；须 NEVER 沿用默认 sink 直写 stderr。sink 须保留 ErrorKind 结构、不得把错误文本当作控制流判断，也不得与 log 后端重复输出。 由 kernel ErrorSink 单测覆盖，不单独扩 BDD step。
