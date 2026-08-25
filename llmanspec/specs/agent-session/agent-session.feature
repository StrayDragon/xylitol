# language: zh-CN
# capability: agent-session
# purpose: Agent 会话管理 — 会话生命周期、能力状态、prompt 构造与回合编排。
# scope: agent 层, protocol 层

功能: agent-session
  背景:
    假如 有一个临时工作目录
    并且 配置了 mock 模型 "test-model"
    并且 工具注册表包含 10 个内置工具

  @req:a1 @human
  场景: agent-session
    - System MUST 提供 agent 编排循环（prompt→model→tools→loop）并发出完整事件流；system prompt MUST 由配置的上下文（AGENTS.md、配置 system prompt、append prompts）组装，MUST NOT 使用硬编码回退。

  @req:a2 @human
  场景: 回合事件
    - System MUST 每轮发出：turn_start、message_start、message_update（流式）、message_end。

  @req:a3 @human
  场景: 工具事件
    - System MUST 每次工具调用在 MessageEnd 之后发出 tool_execution_start，执行期发出至少一次 tool_execution_update（长输出工具宜多段），再 tool_execution_end；MUST NOT 在助手消息流式阶段（MessageEnd 前）发出 tool_execution_start。

  @req:a4 @human
  场景: 模型切换
    - System MUST 支持运行时切换模型（select_model 或等价），切换 MUST 钳制到可用模型集。

  @req:a5 @human
  场景: thinking 级别
    - System MUST 支持按当前模型声明的 thinking_levels 切换档位；显式 set 时目标∉支持集 MUST 拒绝；会话 resume MUST 原样还原末次落盘档名字符串（见 m10），MUST NOT 静默改写历史条目。

  @req:a6 @human
  场景: abort·a6
    - System MUST 支持中止当前 agent 操作（abort 或等价），并清理进行中的回合。

  @req:a7 @human
  场景: 上下文用量
    - System MUST 暴露上下文用量估计（get_context_usage 或等价），返回估计 token 数与上下文窗口百分比。

  @req:a8 @human
  场景: agent-session-store
    - AgentCapabilities 或 ReAct 运行时 MUST 在每条 user、assistant、toolResult 消息完成时（对齐 pi message_end）经 XySessionStore::append_session_entry 写入当前 session；tool 执行结束后 MUST 持久化对应 toolResult；一轮结束后 session 文件或内存 backend MUST 含本轮消息。MUST NOT 仅把对话留在 ReAct 本地 history 而不写入 store。

  @req:a9 @human
  场景: prompt 构造
    - System MUST 构造完整 messages 数组：system prompt + context files + history + user prompt，每轮前置。

  @req:a23 @human
  场景: slash 命令
    - System MUST 提供产品 slash 命令目录（至少含 model、exit、session 等已实现项）及扩展注册命令，与各应用面同源；MUST NOT 以已废弃 pi 短名表冒充产品命令清单；MUST NOT 注册 slash prompt 模板命令（template: 或文件名模板）。

  @req:a24 @human
  场景: 命令分发
    - 以 / 开头的用户输入 MUST 在产品面（idle 提交路径）分发到 slash 命令处理器，MUST NOT 作为普通 prompt 送 LLM；MUST NOT 提供 prompt 模板展开器路径。

  @req:a25 @human
  场景: 资源加载器
    - System MUST 提供 ResourceLoader，聚合项目上下文文件（AGENTS.md、CLAUDE.md）、skills 与 CLI/配置 system prompt；MUST NOT 将 prompts/*.md 发现为可注册 slash 模板。

  @req:as27 @human
  场景: 事件总线生命周期
    - AgentCapabilities MUST 在 turn 生命周期内发出 turn_start 与 turn_end（或等价 XyEvent）；并 MUST 在消息完成时 auto-persist 到 session store（与 a8 一致），不得只依赖无人订阅的旁路。

  @req:as28 @human
  场景: 会话生命周期
    - System MUST 支持创建新会话（start_new_session 或等价，可选 parent 引用）与恢复已有会话（加载并校验，含 CWD 断言）。

  @req:as30 @human
  场景: 内聚分解
    - agent 编排层 MUST 分解为聚焦协作者，覆盖模型管理、工具管理、compaction 编排、skill 激活，而非单一对象混杂无关职责。

  @req:as34 @human
  场景: SessionStore 端口
    - System MUST 经 SessionStore 抽象做会话持久化：protocol 定义端口、infra 提供持久化与内存实现（供测试）、agent 层经该抽象持有 store。

  @req:as35 @human
  场景: 导出 IO 端口
    - 会话导出/导入 MUST 经注入的导出 I/O 端口做文件 I/O（不得直接调用 std::fs），使导出实现无文件系统副作用，未来后端（gist、S3）可替换而不改导出逻辑。

  @req:as38 @human
  场景: 可选 bash 导出
    - 未配置 bash 执行或导出后端时，对应操作（execute_bash / 导出 / 导入）MUST 返回清晰的未配置错误；最小对话 agent 不必强制携带 bash 或导出能力。

  @req:as43 @human
  场景: trust 不在 agent 层
    - 项目 trust 决策 MUST 经 infra trust store + 应用面（Driver / `/trust`）路径；agent 层 MUST NOT 暴露 save_trust_decision 或 agent 层 trust 模块（曾为无生产调用方的预留自由函数，已删除）。

  @req:as45 @human
  场景: 从 store 播种 history
    - AgentRuntime::run_with_id（或等价入口）在已有 session 上开新一轮时 MUST 经 leaf 分支路径、经与 pi buildContextEntries 同构的 compaction-aware 裁切（取路径上最新 CompactionEntry：保留该摘要，并仅保留 firstKeptEntryId 起至该 compaction 前的条目及 compaction 之后的条目；无 compaction 则保留整条 leaf），再经统一 SessionEntry→AgentMessage 投影灌入 history（含 type=message 内的 LLM/Env 角色，以及 compaction/branchSummary 投影；bang-bash 仅接受 message+role=bashExecution），再追加本轮 user prompt；MUST 尊重 exclude_from_context。MUST NOT 把 firstKept 之前已被摘要的消息再送入工作 history；MUST NOT 每轮从空 history 起步；MUST NOT 仅过滤 Message 行而永久丢弃合法 bang-bash 或摘要上下文；MUST NOT 将旧顶层 type=bashExecution/bash_execution 提升为合法 history（此类行由 agent-session-store s20 跳过）。

  @req:as46 @human
  场景: 持久化带 tag 的 agent 部件
    - ReAct/session 将 AssistantMessage/UserMessage 持久化为 SessionEntry::Message 时 MUST 写出 domain-message dm1 的 tagged content；从 store 恢复为 AgentMessage（as_agent_message 或等价）时 MUST 只接受 tagged 部件；遇旧 untagged/裸字符串 content MUST NOT 静默当成合法对话上下文（MUST 跳过该条目并可观测，对齐 agent-session-store s20），MUST NOT 把 thinking 与 text 糊成单一 Text。

  @req:as47 @human
  场景: compaction-aware session context
    - build_session_context / build_session_context_v2（及等价 LLM history 播种）MUST 对 leaf 分支路径应用与 pi buildContextEntries 同构的裁切：存在 CompactionEntry 时仅保留路径上最新一条 compaction 摘要 + firstKeptEntryId 起至该条之前的条目 + 该条之后的条目；MUST NOT 把 firstKept 之前已被摘要的消息再送入 LLM 上下文。

  @req:as48 @human
  场景: resume-import-provider-prefix-path
    - resume_session / import_from_jsonl（或等价加载）后用于打模型的 history MUST 经与同进程续跑相同的唯一路径：SessionEntry → as_agent_message（或等价）→ project_for_llm → ResponsesAssembler；有损 Env 折叠文案 MUST 形状稳定。MUST NOT 在 infra 另建平行 Env 折叠；MUST NOT 无显式 change 改稳定折叠串。system date 日界见相邻 c1905；tools 冻表见 c1900。由单测与维护 lab 覆盖，MUST NOT 强制新 BDD step。
  @executable @req:a2
  场景: text-response
    假如 mock 模型返回文本 "你好，我是一个AI助手"
    当 启动 agent 会话并发送提示 "打个招呼"
    那么 响应事件流包含 TextDelta "你好"
    并且 turn_end 事件触发

  @executable @req:a3
  场景: tool-call
    假如 mock 模型返回工具调用 "read" 参数 {"path":"src/main.rs"}
    并且 read 工具返回 "hello world"
    当 启动 agent 会话并发送提示 "读取文件"
    那么 tool_execution_start 事件触发
    并且 tool_execution_end 事件包含结果 "hello world"
    并且 turn_end 事件包含 toolResult

  @executable @req:a2
  场景: turn-order
    假如 mock 模型返回文本 "分析完成"
    当 启动 agent 会话
    那么 事件按顺序为: turn_start, message_start, message_update, message_end, turn_end

  @executable @req:a5
  场景: thinking-switch
    假如 当前思考级别为 "medium"
    当 切换思考级别到 "high"
    那么 getThinkingLevel 返回 "high"
    并且 thinking_level_change 记录写入会话

  @executable @req:a5
  场景: thinking-clamp
    假如 当前模型不支持思考
    当 尝试将思考级别设为 "high"
    那么 实际思考级别为 "off" 或 set 被拒绝且保持 off

  @executable @req:a7
  场景: context-usage
    假如 会话包含 20000 个 token 的消息
    并且 当前模型上下文窗口为 200000
    当 调用 getContextUsage
    那么 返回 tokens 约为 20000
    并且 percent 约为 10

  @executable @req:as27
  场景: auto-persist
    假如 一个 turn 完成
    当 加载会话文件
    那么 该 turn 的消息记录已保存

  @executable @req:a1
  场景: prompt-from-loader
    假如 项目含 AGENTS.md 且 CLI 组合根构造 agent
    当 agent 构建 system prompt
    那么 system prompt MUST 含 AGENTS.md 内容且 loader 有值时 MUST NOT 回退硬编码占位符

  @executable @req:a2
  场景: turn-events
    假如 agent 处理含工具调用的回合
    当 回合开始
    那么 事件按序发出：turn_start message_start message_update* message_end turn_end

  @executable @req:a3
  场景: tool-stream
    假如 bash 工具流式输出
    当 tool_execution_start 触发
    那么 多次 tool_execution_update 后 tool_execution_end

  @executable @req:a4
  场景: switch-model
    假如 agent 运行中
    当 调用 cycleForward
    那么 下一可用模型成为活动模型

  @executable @req:a5
  场景: thinking-toggle
    假如 模型支持 thinking
    当 变更 thinking level
    那么 新级别钳制到模型能力

  @executable @req:a6
  场景: abort
    假如 agent 流式响应中
    当 调用 abort()
    那么 agent 循环终止并返回 abort 错误

  @executable @req:a8
  场景: persist-user-assistant
    假如 已绑定稳定 session_id 的 Agent 跑完一轮 user→assistant
    当 load_entries(session_id)
    那么 含本轮 user 与 assistant 的 SessionEntry::Message

  @executable @req:a8
  场景: persist-tool-result
    假如 一轮含工具调用
    当 工具执行结束
    那么 store 含对应 toolResult（或等价 tool）消息条目

  @executable @req:a9
  场景: prompt-build
    假如 system prompt 已配置上下文文件
    当 agent 开始回合
    那么 messages 数组为 system prompt、history、user message

  @executable @req:a23
  场景: product-names-in-get-commands
    假如 空扩展命令的能力聚合体
    当 调用 get_commands
    那么 含 session-tree 且不含短名 tree 作为内建主名

  @executable @req:a24
  场景: slash-dispatch
    假如 用户发送 /compact
    当 prompt 被拦截
    那么 compact 处理器被调用

  @executable @req:a24
  场景: no-template-dispatch
    假如 用户发送 /review 及参数且仅存在 prompts/review.md
    当 prompt 处理
    那么 MUST NOT 将 prompt 模板展开并送 LLM

  @executable @req:a25
  场景: context-files-found
    假如 cwd 树存在 AGENTS.md
    当 调用 load_context_files
    那么 以 AGENTS.md 内容为首项返回

  @executable @req:as27
  场景: auto-persist-on-message-end
    假如 已启用 session 的 Agent
    当 assistant message_end 发生
    那么 该消息已 append 到 session store

  @executable @req:as28
  场景: resume-validates-cwd
    假如 会话文件 cwd 指向存在目录
    当 调用 resume_session
    那么 会话加载成功

  @executable @req:as30
  场景: responsibilities-separated
    假如 AgentCapabilities 与 SessionExporter 已构造
    当 分别调用 get_context_usage 与 export_to_html 入口
    那么 各 API 可独立调用且不 panic

  @executable @req:as35
  场景: export-io-injected
    假如 构造含 MockExportIo 的 Agent
    当 调用 export_to_html
    那么 MockExportIo.write 被调用且 agent/ 源码无 std::fs 引用

  @executable @req:as38
  场景: no-bash-configured
    假如 构建无 bash executor 的 agent
    当 调用 execute_bash
    那么 返回提及 bash executor 未配置的错误且不 panic

  @executable @req:as45
  场景: second-turn-sees-first
    假如 同一 session_id 第一轮 user/assistant 已在 store
    当 第二轮 run_with_id
    那么 送给模型的 history 含第一轮消息

  @executable @req:as45
  场景: seed-includes-bash-and-summaries
    假如 session 含 bang bashExecution（Message 内）与 compaction 条目且未 exclude
    当 run_with_id 播种 history
    那么 history 含折叠后的 bash/摘要上下文而非空跳过

  @executable @req:as46
  场景: persist-load-thinking
    假如 含 ThinkingDelta 与 TextDelta 的 assistant 已 persist
    当 load_entries 后 as_agent_message
    那么 content 含独立 Thinking 与 Text 且 type 字段正确

  @executable @req:as46
  场景: legacy-content-rejected
    假如 JSONL message.content 为旧 untagged 形态
    当 as_agent_message 或恢复上下文
    那么 不产生糊成一体的合法 Assistant Text；失败或跳过可观测
