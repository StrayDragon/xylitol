# language: zh-CN
功能: 可选 OTLP 出口
  作为 开发者
  我想要 默认不向远端发送 traces，仅在显式配置时经 OTLP/HTTP 导出
  以便 在 Langfuse 未就绪时仍可安全运行，并在需要时对接标准观测后端

  @req:otel1
  场景: 默认配置不出口 OTLP
    假如 AppConfig 未配置 otel 或 exporter 为 none
    当 进程完成观测装配
    那么 不得安装向远端发送的 OTLP Reporter

  @req:otel2
  场景: 显式 otlp-http 才导出
    假如 feature otel 已启用且 exporter 为 otlp-http 且 endpoint 与认证合法
    当 进程完成观测装配并产生 fastrace span
    那么 经 fastrace-opentelemetry 以 OTLP/HTTP 导出且不得以 gRPC 作为 Langfuse 默认路径

  @req:otel3
  场景: 坏配置降级不阻断
    假如 feature otel 已启用但 otlp-http 的 endpoint 或认证不完整或 exporter 构建失败
    当 进程启动并装配观测
    那么 降级为不收集 OTLP 且对话主路径仍可继续

  @req:otel4
  场景: 出口不引入 tracing 双栈
    假如 仓库启用 otel 出口相关依赖
    当 检查 Cargo 与观测装配路径
    那么 时间线仍为 fastrace 且不得依赖 tracing 或 tracing-subscriber，且不得向 stdout 或 stderr 打印破坏 TUI

  @req:otel5
  场景: 本地与 OTLP 可同时开启
    假如 本地 provider-trace FileReporter 与 OTLP Reporter 均满足开启条件
    当 一批 SpanRecord 被报告
    那么 fan-out 同时投递两侧且 OTLP 侧失败时按降级处理而不静默关掉本地闸

  @req:otel6
  场景: 根 span 始终带 session uuid
    假如 低频观测 span 已激活且当前会话 UUID 已知
    当 创建 react.turn 或 provider.request 等根 span
    那么 属性含 langfuse.session.id 且值等于该 UUID 且不得用 display name 顶替

  @req:otel7
  场景: 有 display name 才写 session_name 元数据
    假如 当前会话已设置 display name
    当 创建低频根 span
    那么 额外含 langfuse.trace.metadata.session_name；若尚未命名则不得写入该键且 session id 不变

  @req:otel8
  场景: 根 span 带 Langfuse observation 类型
    假如 低频观测 span 已激活
    当 创建 provider.request、react.turn、tool.execute
    那么 分别标记 generation、agent、tool，且默认不附带完整 prompt 或 completion 载荷
