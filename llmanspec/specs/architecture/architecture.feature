# language: zh-CN
# managed by llman sdd partition-migrate
功能: architecture

  @req:ar01
  场景: pi-refs-cleared
    假如 src/ 文件含 'Aligns with pi' 注释
    当 运行 rg 'Aligns with pi' src/
    那么 零匹配

  @req:ar02
  场景: aggregate-decomposed
    假如 Agent（原 AgentSession）已增至 25 字段，混合 export bash permission stats trust
    当 变更已应用
    那么 这些关注点位于命名协作者对象，Agent 聚合保持内聚而非无界增长

  @req:ar03
  场景: macro-eval-doc
    假如 llmanspec/changes/c240-consolidate-architecture/ 已打开
    当 运行 macro 评估任务
    那么 docs/architecture/macro-registration.md 存在，含 #[tool]、#[command]、#[provider] 的 ROI 分析

  @req:ar04
  场景: domain-map-updated
    假如 变更后检查 src/domain/
    当 consolidated types 存在
    那么 无重复 CompactionConfig 或 XyFinishReason

  @req:ar05
  场景: app-layer-module-map-exists
    当 变更后检查源码树
    那么 src/app 存在且含 core/ 子层，src/interactive 与 src/server 不再存在，src/protocol 为目录

  @req:ar06
  场景: curated-export
    假如 准备精选 pub use
    当 审查 lib.rs 导出列表
    那么 仅端口与 XyEvent/XyChunk 等契约类型带 Xy；Driver 与 AgentMessage 可不带

  @req:ar07
  场景: no-duplicate-usage
    假如 代码库搜索 token-usage 类型
    当 rg "struct (Usage|XyUsage)"
    那么 恰好剩一个 canonical XyUsage

  @req:ar08
  场景: closed-set
    当 审查 XyEvent 变体与 provider 适配器
    那么 无厂商专名变体；流增量经 XyChunk 进入循环后再发标准 XyEvent

  @req:ar09
  场景: pub-use-exists
    当 检查 src/lib.rs
    那么 存在精选 pub use 且文档标明稳定契约

  @req:r12
  场景: no-domain-jsonschema
    当 rg JsonSchema 于 src/domain
    那么 零匹配

  @req:ar-embed1
  场景: embed-surface
    当 审查 lib 公开 API
    那么 存在嵌入入口且文档列出稳定符号；Driver/bootstrap 可被 crate 外路径引用

  @req:ar-mcp-seam
  场景: no-infra-path
    当 审查 xylitol::embed 与 BootstrappedRuntime
    那么 公开字段/方法签名不出现 infra::mcp::McpServerConfig
