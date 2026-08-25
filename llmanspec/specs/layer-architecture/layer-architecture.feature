# language: zh-CN
# capability: layer-architecture
# purpose: 分层与 seam、Xy 精选导出、嵌入缝与组件粒度：agent / protocol / infra / app（单 crate；无独立 domain 或 runtime_protocol 顶栏；不用源码 grep 元测试卡写法）。
# scope: agent 层与 infra 层与 app 层与 protocol 层, workspace 测试, 分层约定载体（AGENTS）

功能: layer-architecture

  @req:la1 @human
  场景: 核心层与 protocol 布局
    - System MUST 以 agent 层承载编排与投影（ReAct、LLM 投影、session 编排）；agent 侧共享类型 MUST 经 protocol 承载（因 infra 不得依赖 agent）。System MUST 以单一 protocol 层作为跨边界契约 SSOT：wire（Command/Event）与 ports（XyModel、XyTool、XySessionStore、XyEventSink 等）为契约子树；进 port/wire 签名的共享类型（AgentMessage、Env、SessionEntry、XyEvent、XyChunk 等）MUST 位于 protocol 根共享区（可按领域聚子树，非独立 vocab/types 顶栏）；wire MUST NOT 依赖 ports；ports MAY 依赖根共享类型；根共享类型 MUST NOT 依赖 ports/wire/agent/infra。MUST NOT 提供独立的 domain、runtime_protocol 或第三顶栏子树。

  @req:la2 @human
  场景: 向下依赖
    - infra 层 MUST NOT 导入任何 agent 符号；agent 层 MUST NOT 导入任何具体 infra 类型（仅经 protocol ports）；应用面（cli 渲染子路径、tui、非组合根的 app 模块）MUST NOT 导入 infra 或 agent 子模块内部，MUST 经 app 跨面缝（composition / Driver / bootstrap / dispatch）与 agent 公共入口导入。组合根（cli 面、server 面、装配模块）可同时导入 agent 与 infra 做装配。InProcessDriver MAY 调用文档化的表面 infra（trust / clipboard / 必要 config 读）；MUST NOT 成为第二套组合根去构造 provider/session 工具集。protocol 层 MUST NOT 依赖 agent 或 infra 实现。分层靠 AGENTS + review + 缝行为测保障，MUST NOT 要求源码 grep 元测试。

  @req:la4 @human
  场景: app 层命名
    - app 表面（用户入口：cli、server、tui、gui）与跨面缝（composition、driver）MUST 分离：缝位于 app 跨面缝区，表面不托管缝实现。print 模式 MUST 为 CLI 子模式，而非顶层 app 模块。

  @req:la6 @human
  场景: server 应用面
    - System MUST 提供 server 应用面，托管 agent + infra 运行时，并经四象限信封暴露产品契约：unary 与 respond 为 HTTP POST，下行为 WebSocket 且 MUST NOT 收业务上行。同一 {addr,port} 占用 MUST 失败，MUST NOT 整机锁文件，MUST NOT port+1。MUST NOT 以 /api/v1 REST 资源动词或全双工 WS 应用帧为产品真源。

  @req:la7 @human
  场景: 运行时归属
    - 全部运行时（provider、tool、session storage、exec-env、bash executor、secret resolution、resource loader、export I/O）MUST 位于 infra 层；agent 层 MUST 仅经 protocol 端口注入持有（组合根装配），MUST NOT 直接调用 infra 构造器或借用 infra exec 原语。

  @req:la8 @human
  场景: agent 独立性
    - Agent（能力聚合体）构造器 MUST NOT 要求 Session、MUST NOT 持有 session_id、MUST NOT 依赖 Session 持久化生命周期；session_id 为 app 层绑定的 SessionStore 查询键。

  @req:la9 @human
  场景: 分层保障方式
    - 分层不变量（la2 依赖方向）的保障手段为仓库自身约定：架构 AGENTS 文档、l8ng-write-surface 流程与缝/行为测试；System MUST NOT 要求或恢复以扫描 import 路径为手段的 arch_guard 测试模块作为强制闸（新增分层约束优先写成缝行为测或 AGENTS 约定）。

  @req:la13 @human
  场景: agent 薄且无展示
    - agent/ MUST NOT 托管纯展示或用户指引逻辑（如静态帮助文本或消息格式化）；此类逻辑 MUST 位于 app/（客户端表面），agent/ MUST 仅含编排循环、hook 点与编排状态。

  @req:la15 @human
  场景: app 组合根
    - System MUST 提供单一组合根装配入口，从构建选项集中构造完整接线的 Agent，使 CLI、Server 与 TUI 不重复 Agent 接线代码。

  @req:la17 @human
  场景: Xy 协议端口
    - Xy 前缀端口契约 MUST 位于 protocol 层端口区；其签名所用共享类型 MUST 来自 protocol 层根共享区。

  @req:la19 @human
  场景: 缝与表面分离
    - app 层 MUST 将跨面缝（组合根与运行时 driver 边界）分组于独立跨面缝区，与自包含应用表面（cli、server、tui、gui）分离；应用表面 MUST NOT 直接托管 build_agent 接线或 Driver trait（或等价）。

  @req:la20 @human
  场景: 共享 bootstrap 与 dispatch
    - System MUST 提供共享 bootstrap 入口集中从 CLI args 到运行时聚合体的完整装配路径，以及共享 dispatch 入口集中 Command 变体执行语义；print（cli）、server、tui MUST 调用 bootstrap，tui MUST 调用 dispatch，任何表面 MAY NOT 内联自有装配或命令分发实现。装配步骤清单（config load、ModelRegistry build、trust resolution、resource discovery 含 templates/context_files/system_prompt/append_system_prompt、compaction settings、permission、build_agent、model select）随代码组织演化，由架构 AGENTS 维护。

  @req:la21 @human
  场景: 禁止死包装协作者
    - agent 层 MUST NOT 长期保留对端口的纯转发壳（无附加状态或行为，且无生产调用方）；死码分诊与删除策略由架构 AGENTS 体量策略与 l8ng-audit-dead-code skill 承载。

  @req:la-embed1 @human
  场景: 嵌入缝与公开入口
    - System MUST 向库用户提供文档化的嵌入入口（含 bootstrap/Driver 或等价），使外部 crate 无需 reach-in agent::capabilities 或 infra 具体类型即可装配并驱动一轮对话。嵌入路径 MUST 仅依赖精选 Xy 契约与文档化应用缝；MUST NOT 要求嵌入方 import infra 具体实现或 agent 子模块内部类型以完成标准装配。

  @req:la-server-driver @human
  场景: Server 与 Print 同缝
    - Server 与 Print MUST 共享同一应用缝（Driver/bootstrap）；新增 server 能力时优先扩缝，禁止为 server 单独复制一套 agent 编排。

  @req:la-mcp-seam @human
  场景: MCP 经嵌入缝
    - System MUST 向嵌入方提供不依赖 infra 路径的 MCP 服务器描述（或等价句柄），使 bootstrap/reload 可在不 reach-in infra 实现的情况下传递配置。缝边界内的 MCP 会话类型（或等价）MUST 将嵌入缝 MCP 描述转为 infra 实现；应用面 MUST NOT 为 MCP 装配单独 reach-in infra 类型。

  @req:la-dispatch-consume @human
  场景: dispatch 有消费方
    - 跨面命令执行缝 dispatch MUST 至少被一个生产应用面消费（Server 或 TUI）；仅单测覆盖而无面接线视为逻辑死码，死码分诊与激活策略由架构 AGENTS 与 l8ng-audit-dead-code skill 承载。

  @req:la25 @human
  场景: provider 包开闭
    - LLM 厂商差异 MUST 收敛在 packages/xylitol-ai-bridge 的 adapter 族；主仓投影缝（agent 内 project_for_llm）仅负责 Env→LLM 折叠。新 OpenAI-like 或 Anthropic-like 兼容端 MUST 以开闭方式扩展（新 adapter/配置），MUST NOT 为某一网关修改 AgentMessage 角色集或 agent ReAct 核心；agent MAY 依赖 bridge DTO 组合 Llm 臂；agent/infra MUST NOT 依赖厂商 HTTP/SDK。

  @req:la26 @human
  场景: feature-flags
    - System MUST 为每个可选能力定义带域前缀的 feature flag，使非默认 feature 零开销编译剔除。内置能力（tools、hooks、security、print-mode）无 feature flag，运行时由 config.yaml 控制。ACP 模式使用 feature flag infra-acp。

  @req:la27 @human
  场景: minimal-default-features
    - Cargo.toml default MUST 为 cli + tui + otel + server（个人 agent 开箱含本机 Host 监听器能力）；MUST NOT 要求 default 包含全部非 dev feature。构建 feature 闸与条件编译以 Cargo.toml 为 SSOT（原 build-config 并入）。

  @req:ar01 @human
  场景: 清除 pi 文档引用
    - 源文件 doc comments MUST NOT 引用 pi coding agent 或其 TypeScript 模块；pi 设计理由引用 MUST 替换为该模块实际职责的独立描述。文档卫生清单（含存量清理跟进）由架构 AGENTS 维护。

  @req:ar07 @human
  场景: 消除重复领域实体
    - System MUST NOT 为同一领域概念定义多种类型；Usage、StopReason 与 Compaction 配置 MUST 各有一个 canonical 类型。

  @req:ar09 @human
  场景: 精选 pub use 清单
    - System MUST 在库公开入口（或等价）维护精选 pub use，至少覆盖 XyModel、XyTool、XySessionStore、XyEvent、XyChunk 及文档化的配套错误/配置类型；未列入清单的内部模块 MUST NOT 被误当作稳定公开 API。

  @req:r12 @human
  场景: 配置 JSON Schema 边界
    - 需要 JSON Schema 的配置/settings DTO MUST 收敛于 infra 配置边界（config/settings 或等价）并在该处 derive schemars；agent/protocol 会话词汇 MUST NOT 仅为 schema 而 derive JsonSchema。

  @req:la-cs1 @human
  场景: 双角色
    - System MUST 把运行时分成 client 与 host 两个产品角色（职责划分，MUST NOT 解读为必须拆成两个进程）。client MUST 只承担面本地：键盘、绘制、TTY、本机编辑器、剪贴板。host MUST 承担模型、会话生命周期、资源装配（配置 / MCP / 工具 / 技能 / prompt）、工作区执行（含人发起的 bang）与项目信任。

  @req:la-cs2 @human
  场景: 归属判据
    - 依赖工作区、仓库、模型或 MCP 的能力 MUST 由 host 执行。依赖本机终端或本机硬件的能力 MUST 由 client 执行。导出 MUST 由 host 序列化内容、由 client 写入本机路径；导入对称。一条 reload MUST 同时让 client 热加载本机键位与主题，并向 host 请求重装 MCP / prompt / 技能。默认 embed 同进程时，导出/导入/reload MUST 视为已满足（同一进程内按角色分工即可），MUST NOT 要求先存在跨进程 attach。

  @req:la-cs3 @human
  场景: embed 不要求监听器
    - host 是操作器角色，MUST NOT 被等同于占用了网络绑定的监听器。print 与库嵌入 MUST 允许 client 与 host 同进程、同一条产品契约自连，且 MUST NOT 要求先占用监听地址。占用绑定是显式 serve 的职责，不是 embed 的前提。本条 MUST NOT 解读为产品 TUI 可以无监听器启动。

  @req:la-cs4 @human
  场景: session 原子与分面
    - 产品最小对话单元 MUST 是 session。cwd MUST 只表示执行面（工具 / bang / trust / MCP 池），MUST NOT 当作唯一分类夹。视图组织 MUST 允许用 tag 等高频分面，且分面索引 MUST 与 transcript 分离。本 requirement MUST NOT 要求交付独立 tag catalog。

  @req:la-cs5 @human
  场景: 一 session 一写者
    - 同一 session MUST 至多一个写者。对已被写入的 session 再 attach MUST 只读恢复。只读面上发起写入 MUST 失败并说明已有其它客户端以写者连接。尚未提供多客户端 attach 时，单进程默认路径 MUST 视为已满足本规则。

  @req:la-cs6 @human
  场景: 产品 TUI 要求监听器
    - 产品 TUI MUST 要求本机 Host 监听器已在听（默认 127.0.0.1:18790，可覆盖）。未在听 MUST 失败。print 与库嵌入 MUST 仍允许无网络绑定。
