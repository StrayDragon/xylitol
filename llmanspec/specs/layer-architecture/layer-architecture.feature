# language: zh-CN
# managed by llman sdd partition-migrate
功能: layer-architecture

  @req:la1
  场景: shared-types-in-domain
    假如 agent/ 与 infra/ 均需纯数据类型
    当 声明并导入类型
    那么 两层均从 crate::domain 导入且 runtime_protocol 从 crate::runtime_protocol 导入

  @req:la2
  场景: app-layer-isolated
    当 src/app/core/driver.rs 新文件 import crate::infra::tools::BashTool
    那么 架构 guard 失败并报告违规

  @req:la2
  场景: composition-root-allowed
    当 src/app/core/composition.rs 文件 import crate::infra::tools::default_tools
    那么 架构 guard 允许为文档化组合根

  @req:la3
  场景: guard-catches-violation
    假如 src/infra 新文件添加 use crate::agent::session
    当 架构 guard 运行
    那么 guard 失败并报告违规文件路径

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
  场景: guard-scans-app
    当 文档化缝外 app/ 新文件 import crate::agent
    那么 架构 guard 失败并报告违规文件路径

  @req:la10
  场景: note-greppable
    假如 代码中做刻意简化
    当 作者标记
    那么 行含 '// NOTE: ... ceiling: ... upgrade: ...' 且可 grep

  @req:la11
  场景: driver-is-clean
    当 扫描 app/core/driver.rs 的 infra import
    那么 不含 crate::infra import

  @req:la12
  场景: allowlist-blocks-new-violations
    假如 agent 文件添加允许列表外的新 crate::infra import
    当 guard 运行
    那么 构建失败且消息指明文件不在允许列表

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
    当 CLI、RPC、Server 均需 Agent
    那么 三者均调用 app::core::composition::build_agent 而非重复接线

  @req:la16
  场景: xy-domain-types
    假如 公共 domain 类型对外
    当 应用变更后
    那么 重命名为 Xy 并一致导入

  @req:la17
  场景: xy-protocol-ports
    假如 引用 runtime_protocol trait
    当 应用变更后
    那么 重命名为 Xy 并由 infra 实现

  @req:la18
  场景: event-layer-check
    假如 定位事件类型与端口
    当 检查 src
    那么 XyEvent 在 domain 且 XyEventSink 在 runtime_protocol

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

  @req:la-server-driver
  场景: shared-seam
    当 对比 print 与 server 装配
    那么 二者均经 composition/bootstrap 与 Driver 语义

  @req:la-mcp-seam
  场景: composition-owns
    当 审查 McpSession::reload 签名
    那么 参数为缝类型；infra 转换仅在 composition/infra 内

  @req:la-dispatch-consume
  场景: server-wires
    当 c550 归档后审查调用点
    那么 server REST 生产路径调用 dispatch

  @req:la25
  场景: new-compatible-endpoint
    假如 新增兼容 OpenAI base_url 的供应商
    当 接线
    那么 不改 AgentMessage 定义

  @req:la25
  场景: domain-no-sdk
    假如 src/domain 与 src/agent
    当 rg async_openai 或 anthropic SDK
    那么 零匹配
