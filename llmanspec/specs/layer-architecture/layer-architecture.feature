# language: zh-CN
# managed by llman sdd partition-migrate
功能: layer-architecture

  @req:la1
  场景: shared-types-in-agent-and-protocol
    假如 agent/ 与 infra/ 需要跨边界类型或端口
    当 声明并导入
    那么 共享类型自 crate::protocol（根模块或 agent/crate 再导出）导入；ports 自 protocol::ports；线协议自 protocol::wire
    并且 不存在独立 src/domain/ 或 src/runtime_protocol/ 顶栏

  @req:la2
  场景: app-layer-via-seam
    当 应用面需要工具或会话能力
    那么 经 app/core Driver 或 composition 缝，而非直接 import crate::infra 生产类型

  @req:la2
  场景: composition-root-allowed
    当 src/app/core/composition.rs 文件 import crate::infra::tools::default_tools
    那么 允许为文档化组合根

  @req:la3
  场景: infra-no-agent-import
    假如 审查 src/infra 生产代码
    当 查找 crate::agent 引用
    那么 无生产 import；由约定与审查保障而非 arch_guard 元测试

  @req:la4
  场景: app-dir-exists
    当 重构后检查源码树
    那么 src/app 存在且 src/interactive 不再存在

  @req:la5
  场景: protocol-commands
    假如 client 向 core 发送命令
    当 命令序列化
    那么 反序列化为 protocol/ 中定义的 protocol::Command 枚举

  @req:la6
  场景: server-dir-moved
    当 重构后检查源码树
    那么 src/app/server 存在且 src/server 不再存在

  @req:la7
  场景: executor-not-in-agent
    假如 定位 bash executor
    当 检查源文件
    那么 位于 infra/ 且 agent/ 仅经注入端口访问

  @req:la8
  场景: facade-wording-reconciled
    假如 la8 曾引用 Agent facade 构造器
    当 变更后阅读要求
    那么 指 Agent 能力聚合体构造器且仍禁止要求 Session 或持有 session_id

  @req:la9
  场景: no-source-grep-arch-guard
    当 审查 src/tests.rs
    那么 不存在 arch_guard 模块或以扫描 import 路径为手段的分层元测试

  @req:la10
  场景: note-greppable
    假如 代码中做刻意简化
    当 作者标记
    那么 行含 '// NOTE: ... ceiling: ... upgrade: ...' 且可 grep

  @req:la11
  场景: driver-surface-infra-ok
    当 审查 InProcessDriver 的 trust 或 clipboard 路径
    那么 允许经 Driver 调表面 infra；应用面仍不直接 import infra

  @req:la12
  场景: no-arch-guard-regression
    假如 提议用源码 grep 恢复分层闸
    当 对照 la9/la12
    那么 MUST 拒绝；改用缝行为测或 AGENTS 约定

  @req:la13
  场景: auth-guidance-not-in-agent
    假如 检查源码树中的用户指引消息格式化函数
    当 扫描 src/agent/
    那么 agent/ 下无纯展示或静态帮助文本模块（如原 agent::auth guidance）

  @req:la14
  场景: no-shim
    假如 在源码树 grep 向后兼容 shim
    当 grep 运行
    那么 零匹配 pub use domain as core 或类似再导出

  @req:la15
  场景: composition-reused
    当 CLI、Server、TUI 均需 Agent
    那么 三者均调用 app::core::composition::build_agent 而非重复接线

  @req:la16
  场景: xy-shared-types
    假如 公共跨层类型对外
    当 应用变更后
    那么 重命名为 Xy 并一致导入（归属 agent/ 或 protocol/）

  @req:la17
  场景: xy-protocol-ports
    假如 引用 protocol 端口 trait
    当 应用变更后
    那么 重命名为 Xy 并由 infra 实现；路径为 crate::protocol

  @req:la18
  场景: event-layer-check
    假如 定位事件类型与端口
    当 检查 src
    那么 XyEvent 在 protocol/lifecycle（或精选 pub use）且 XyEventSink 在 protocol/ports；wire Event 与之分离

  @req:la18
  场景: closed-set
    当 审查 XyEvent 变体与 provider 适配器
    那么 无厂商专名变体；流增量经 XyChunk 进入循环后再发标准 XyEvent

  @req:la19
  场景: seams-under-core
    当 检查源码树
    那么 composition.rs 与 driver.rs 位于 app/core/，且无 Driver trait 或 build_agent 缝直接位于表面目录（cli/、server/、tui/、gui.rs）

  @req:la20
  场景: bootstrap-agent-runtime
    假如 bootstrap 完成
    当 检查 BootstrappedAgent 字段
    那么 持有 AgentRuntime（或经 Driver 封装）

  @req:la21
  场景: no-session-io
    当 搜索 SessionIO 与 PermissionGate 生产调用
    那么 类型已删除或不再作为无逻辑壳存在

  @req:la-embed1
  场景: no-reach-in
    当 按嵌入文档装配 agent
    那么 调用点不出现 crate::infra:: 或 agent::session:: 路径（测试双除外）

  @req:la-embed1
  场景: embed-surface
    当 审查 lib 公开 API
    那么 存在嵌入入口且文档列出稳定符号；Driver/bootstrap 可被 crate 外路径引用

  @req:la-server-driver
  场景: shared-seam
    当 对比 print 与 server 装配
    那么 二者均经 composition/bootstrap 与 Driver 语义

  @req:la-mcp-seam
  场景: composition-owns
    当 审查 McpSession::reload 签名
    那么 参数为缝类型；infra 转换仅在 composition/infra 内

  @req:la-mcp-seam
  场景: no-infra-path
    当 审查 xylitol::embed 与 BootstrappedRuntime
    那么 公开字段/方法签名不出现 infra::mcp::McpServerConfig

  @req:la-dispatch-consume
  场景: server-wires
    当 c550 归档后审查调用点
    那么 server REST 生产路径调用 dispatch

  @req:la25
  场景: new-compatible-endpoint
    假如 新增兼容 OpenAI base_url 的供应商
    当 接线
    那么 不改 AgentMessage 的 Env/LLM 角色集（仅新 adapter/配置）

  @req:la25
  场景: agent-no-sdk
    假如 src/agent 与 src/protocol
    当 rg async_openai 或 anthropic SDK
    那么 零匹配（允许依赖 bridge DTO，禁止 SDK）

  @req:ar01
  场景: pi-refs-cleared
    假如 src/ 文件含 'Aligns with pi' 注释
    当 运行 rg 'Aligns with pi' src/
    那么 零匹配

  @req:ar02
  场景: aggregate-decomposed
    假如 Agent 已增至 25 字段，混合 export bash permission stats trust
    当 审查聚合边界
    那么 这些关注点位于命名协作者对象，Agent 聚合保持内聚而非无界增长

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

  @req:ar09
  场景: pub-use-exists
    当 检查 src/lib.rs
    那么 存在精选 pub use 且文档标明稳定契约

  @req:r12
  场景: no-domain-jsonschema
    当 rg JsonSchema 于已删除的 src/domain 或现存 agent/protocol 词汇模块
    那么 零匹配（不得给会话词汇挂 JsonSchema）
