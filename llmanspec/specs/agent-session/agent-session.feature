# language: zh-CN
# migrated from tests/features/agent.feature
功能: agent-session
  背景:
    假定 有一个临时工作目录
    并且 配置了 mock 模型 "test-model"
    并且 工具注册表包含 7 个内置工具

  场景: text-response
    假定 mock 模型返回文本 "你好，我是一个AI助手"
    当 启动 agent 会话并发送提示 "打个招呼"
    那么 响应事件流包含 TextDelta "你好"
    并且 turn_end 事件触发

  场景: tool-call
    假定 mock 模型返回工具调用 "read" 参数 {"path":"src/main.rs"}
    并且 read 工具返回 "hello world"
    当 启动 agent 会话并发送提示 "读取文件"
    那么 tool_execution_start 事件触发
    并且 tool_execution_end 事件包含结果 "hello world"
    并且 turn_end 事件包含 toolResult

  场景: turn-order
    假定 mock 模型返回文本 "分析完成"
    当 启动 agent 会话
    那么 事件按顺序为: turn_start, message_start, message_update, message_end, turn_end

  场景: thinking-switch
    假定 当前思考级别为 "medium"
    当 切换思考级别到 "high"
    那么 getThinkingLevel 返回 "high"
    并且 thinking_level_change 记录写入会话

  场景: thinking-clamp
    假定 当前模型不支持思考
    当 尝试将思考级别设为 "high"
    那么 实际思考级别被限制为 "low" 或模型支持的最高级别

  @req:a7
  场景: context-usage
    假定 会话包含 20000 个 token 的消息
    并且 当前模型上下文窗口为 200000
    当 调用 getContextUsage
    那么 返回 tokens 约为 20000
    并且 percent 约为 10

  场景: auto-persist
    假定 一个 turn 完成
    当 加载会话文件
    那么 该 turn 的消息记录已保存

  @req:a1
  场景: prompt-from-loader
    假如 项目含 AGENTS.md 且 CLI 组合根构造 agent
    当 agent 构建 system prompt
    那么 system prompt MUST 含 AGENTS.md 内容且 loader 有值时 MUST NOT 回退硬编码占位符

  @req:a2
  场景: turn-events
    假如 agent 处理含工具调用的回合
    当 回合开始
    那么 事件按序发出：turn_start message_start message_update* message_end turn_end

  @req:a3
  场景: tool-stream
    假如 bash 工具流式输出
    当 tool_execution_start 触发
    那么 多次 tool_execution_update 后 tool_execution_end

  @req:a4
  场景: switch-model
    假如 agent 运行中
    当 调用 cycleForward
    那么 下一可用模型成为活动模型

  @req:a5
  场景: thinking-toggle
    假如 模型支持 thinking
    当 变更 thinking level
    那么 新级别钳制到模型能力

  @req:a6
  场景: abort
    假如 agent 流式响应中
    当 调用 abort()
    那么 agent 循环终止并返回 abort 错误

  @req:a8
  场景: persist-user-assistant
    假如 已绑定稳定 session_id 的 Agent 跑完一轮 user→assistant
    当 load_entries(session_id)
    那么 含本轮 user 与 assistant 的 SessionEntry::Message

  @req:a8
  场景: persist-tool-result
    假如 一轮含工具调用
    当 工具执行结束
    那么 store 含对应 toolResult（或等价 tool）消息条目

  @req:a9
  场景: prompt-build
    假如 system prompt 已配置上下文文件
    当 agent 开始回合
    那么 messages 数组为 system prompt、history、user message

  @req:a10
  场景: bdd-pass
    假如 调用 BDD runner
    当 cargo test --test bdd
    那么 全部 agent 场景通过

  @req:a21
  场景: template-expand
    假如 模板含第一参数占位符
    当 以 main.rs 展开模板
    那么 内容中 main.rs 已替换

  @req:a21
  场景: default-value
    假如 模板第一参数有默认值
    当 无参展开模板
    那么 内容含默认值

  @req:a22
  场景: load-global
    假如 prompts 目录有 review.md
    当 加载模板
    那么 返回含 description 的 review 模板

  @req:a23
  场景: product-names-in-get-commands
    假如 空扩展命令的 AgentSession
    当 调用 get_commands
    那么 含 session-tree 且不含短名 tree 作为内建主名

  @req:a24
  场景: slash-dispatch
    假如 用户发送 /compact
    当 prompt 被拦截
    那么 compact 处理器被调用

  @req:a24
  场景: template-dispatch
    假如 用户发送 /review 及参数
    当 prompt 被拦截
    那么 模板展开并送 LLM

  @req:a25
  场景: context-files-found
    假如 cwd 树存在 AGENTS.md
    当 调用 load_context_files
    那么 以 AGENTS.md 内容为首项返回

  @req:a26
  场景: bdd-pass
    假如 调用 BDD runner
    当 运行 cargo test
    那么 全部 agent-v3 场景通过

  @req:as27
  场景: auto-persist-on-message-end
    假如 已启用 session 的 Agent
    当 assistant message_end 发生
    那么 该消息已 append 到 session store

  @req:as28
  场景: resume-validates-cwd
    假如 会话文件 cwd 指向存在目录
    当 调用 resume_session
    那么 会话加载成功

  @req:as29
  场景: bdd-pass
    假如 调用 BDD runner
    当 cargo test --test bdd
    那么 全部 agent-v4 场景通过

  @req:as30
  场景: responsibilities-separated
    假如 枚举 session 模块职责
    当 划定组件边界
    那么 各组件拥有单一职责类

  @req:as31
  场景: capabilities-named
    假如 源码树在本变更后
    当 rg '\bstruct Agent\b' agent/session
    那么 能力聚合体类型名为 AgentCapabilities

  @req:as32
  场景: retry-extracted
    假如 AgentSession 持有 retry_state 与超 100 行内联 auto-retry 逻辑
    当 应用变更后
    那么 存在 AutoRetryEngine 协作者，AgentSession 组合它，公共 retry 行为不变

  @req:as32
  场景: bash-extracted
    假如 AgentSession 持有 bash_cancel 与内联 !cmd/!!cmd 处理
    当 应用变更后
    那么 存在 BashExecHandler 协作者，AgentSession 组合它，infra-bash slash 命令行为一致

  @req:as32
  场景: export-extracted
    假如 AgentSession 有内联导出/导入方法
    当 应用变更后
    那么 存在 SessionExporter 协作者，AgentSession 组合它，导出输出字节一致

  @req:as32
  场景: api-retained
    假如 AgentSession 公共 API（per as31）
    当 session 模块重组为含子模块的目录
    那么 全部既有公共方法签名与发出事件不变

  @req:as33
  场景: no-orphan-managers
    假如 检查 src/agent/
    当 列出顶层文件
    那么 model_manager.rs、compaction_orchestrator.rs、skill_manager.rs 不存在（分别位于 model/、compaction/、prompt/）

  @req:as34
  场景: store-is-trait
    假如 agent 持有 session backend
    当 检查字段类型
    那么 为 Arc<dyn SessionStore> 而非 Arc<SessionManager>

  @req:as35
  场景: export-io-injected
    假如 为导出构造 AgentSession
    当 执行写入
    那么 写入经注入 ExportIo 实现，agent/ 内无直接 fs 调用

  @req:as36
  场景: bang-in-agent
    假如 定位 bang 解析器
    当 检查模块
    那么 定义于 src/agent/session/bang.rs

  @req:as37
  场景: exporter-in-agent
    假如 定位导出渲染辅助
    当 检查模块
    那么 定义于 src/agent/session/export.rs

  @req:as38
  场景: no-bash-configured
    假如 构建无 bash executor 的 agent
    当 调用 execute_bash
    那么 返回提及 bash executor 未配置的错误且不 panic

  @req:as39
  场景: loop-no-dup-state
    假如 检查 AgentLoop 字段
    当 应用变更后
    那么 hooks 与 tool_mode 仅在 AgentSession，AgentLoop 仅持 session 与 cancel

  @req:as40
  场景: agent-session-renamed
    假如 重命名后源码树
    当 rg '\bAgentSession\b' src/ tests/ 运行
    那么 零匹配且 Agent 为能力聚合体

  @req:as40
  场景: snapshot-regenerated
    假如 公共 API 快照曾针对 AgentSession
    当 快照测试运行
    那么 重生成后针对 Agent 类型通过

  @req:as41
  场景: permission-gate-exists
    假如 permission 端口曾是 Agent 裸字段
    当 应用变更后
    那么 PermissionGate 结构持有它且 get_permission 仍返回 Arc 供 react 路由

  @req:as42
  场景: stats-delegate
    假如 get_session_stats 曾含内联聚合
    当 检查 Agent 方法
    那么 主体为一行委托至 stats::compute

  @req:as43
  场景: trust-free-fn
    假如 save_trust_decision 曾在 Agent 上
    当 应用变更后
    那么 为 agent/session/trust.rs 自由函数且 Agent 不再携带

  @req:as44
  场景: dead-methods-gone
    假如 steering 域与其它死方法曾存在
    当 在 src/agent/ rg 那些方法名
    那么 无生产死方法残留（仅合法使用方法保留）

  @req:as45
  场景: second-turn-sees-first
    假如 同一 session_id 第一轮 user/assistant 已在 store
    当 第二轮 run_with_id
    那么 送给模型的 history 含第一轮消息

  @req:as45
  场景: seed-includes-bash-and-summaries
    假如 session 含 bang bashExecution（Message 内）与 compaction 条目且未 exclude
    当 run_with_id 播种 history
    那么 history 含折叠后的 bash/摘要上下文而非空跳过

  @req:as46
  场景: persist-load-thinking
    假如 含 ThinkingDelta 与 TextDelta 的 assistant 已 persist
    当 load_entries 后 as_agent_message
    那么 content 含独立 Thinking 与 Text 且 type 字段正确

  @req:as46
  场景: legacy-content-rejected
    假如 JSONL message.content 为旧 untagged 形态
    当 as_agent_message 或恢复上下文
    那么 不产生糊成一体的合法 Assistant Text；失败或跳过可观测
