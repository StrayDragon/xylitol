# language: zh-CN
# capability: agent-prompt
# purpose: 系统提示组装：skills/context 注入与热应用；默认正文经沙箱 minijinja；产品主路径用 skill（含 $name），MUST NOT 提供 pi 式 slash prompt templates。
# scope: src/agent/prompt/, src/protocol/, src/infra/resource/

功能: agent-prompt

  @req:pt1 @human
  场景: system-assembly
    - System MUST 经 build_system_prompt（或等价）组装系统提示，可包含工具片段、skills 摘要、context 文件；自定义 system 文件可替换或 append。组装顺序 MUST 可文档化为 stable 正文与资源 → skills 元数据 → Guidelines → runtime_policy。产品默认 MUST NOT 将日历日或 CWD 写入系统提示前缀（session_env 另条）；秒级时刻等 volatile 读数 MUST NOT 进入系统提示前缀。

  @req:pt2 @human
  场景: skills-section
    - 当 ResourceLoader 发现 skills 时，系统提示 MUST 能包含 available_skills（或等价）清单供模型发现；未信任项目 MUST NOT 注入项目 skills。skills 元数据属 stable（会话作用域）；SKILL.md 正文仍经 $skill 注入 user 投影，MUST NOT 为对齐外部书语而强制改走状态栏。

  @req:pt3 @human
  场景: no-slash-prompt-templates·pt3
    - System MUST NOT 从 prompts 目录发现或注册 slash prompt 模板命令（含 /template:name 与 pi 式 /filename）；磁盘遗留 prompts/*.md MUST 忽略；MUST NOT 提供 $1/$@ 位置参数模板展开。产品主路径用 skills。

  @req:pt5 @human
  场景: safe-minijinja-default-assembly
    - 默认 system 组装路径（无 SYSTEM.md/custom_prompt 整段替换时）MUST 经沙箱 minijinja 渲染嵌入默认模板：UndefinedBehavior::Strict；上下文仅白名单键（至少含 date、cwd、tools、guidelines、skills、runtime_policy、mcp_discover 或等价）；MUST NOT 注册任意文件系统 loader；MUST NOT 将 env/secret 命名空间注入该上下文。{% include %} MUST 仅解析预注册模板名。SystemPromptOpts（或等价）MUST 允许注入 date（或等价时钟）。由单测覆盖安全与可钉 date；MUST NOT 为静态依赖存在性单独扩重型 BDD（可保留窄正向场景或 feature:false）。

  @req:pt6 @human
  场景: context 热应用
    - Agent MUST 提供 apply_prompt_resources（或等价）：用新的 context_files、system_prompt、append_system_prompt 重建系统提示；MUST 只影响后续 run；MUST NOT 改写已持久化历史消息。

  @req:pt7 @human
  场景: skills 热应用与引导注入
    - Agent MUST 在构造/bootstrap 时将 ResourceLoader 发现的 skills 写入 SystemPromptOpts 并经 build_system_prompt 进入 available_skills（若非空）；MUST 提供 apply_skills（或扩展 apply_prompt_resources）用新目录重建系统提示；MUST 只影响后续 run；MUST NOT 改写已持久化历史；未信任 MUST NOT 注入项目 skills。

  @req:pt8 @human
  场景: dollar-skill 提交注入
    - 当用户消息含 $name 且 name 在已加载 skills 目录中时，System MUST 在发往模型前将对应 SKILL.md 正文（去 frontmatter）注入该 user 消息的模型侧投影；会话历史 MUST 保留原始含 $ 文本；未知 $name MUST 透传；未信任 MUST NOT 注入项目 skill 正文（目录已空即可）。

  @req:pt9 @human
  场景: tool-prompt-guidelines-assembly
    - set_tools 或等价初次装配工具集时，System MUST 收集各 XyTool::prompt_guidelines（非空）写入 SystemPromptOpts.prompt_guidelines，并经 build_system_prompt 进入 Guidelines 段；内置 bash/read/edit/write（及已启用的同族工具）MUST 提供与 pi 对齐的非空 guideline 短句。MUST NOT 为抑制 XML 伪工具而注入防呆文案。自定义 SYSTEM.md 或 custom_prompt 整段替换默认正文时 MUST 保持替换语义（不偷偷回填 Available tools）。

  @req:pt10 @human
  场景: runtime-policy-fragments
    - System MUST 按当前工具批模式（XyBatchMode）从内置表解析 runtime policy 片段并经 build_system_prompt 注入 <runtime_policy> 段（位于 APPEND_SYSTEM / Guidelines 之后；产品默认其后不再追加 date/cwd）；barrier_parallel MUST 注入多-tool 同消息策略片段；sequential MUST NOT 注入该片段；set_tool_mode（或等价）MUST 在下一轮重建前提示中反映片段启停。Session MUST 持有已应用 fragment id 集合：同一 id 集合 MUST NOT 重复同步/重写片段正文（同模式重复 set_tool_mode 不得导致 <runtime_policy> 重复段或同 body 多份）；模式变更后 id 集合变化时 MUST 再同步一次。MUST NOT 要求用户手写 APPEND_SYSTEM 才能获得该策略。由单测覆盖，MUST NOT 为静态存在性单独扩 BDD step。

  @req:pt11 @human
  场景: builtins-available-tools-mcp-discover
    - 默认 build_system_prompt 路径下 Available tools 散文清单 MUST 仅含内置（非 mcp__ / 过渡 mcp- / mcp_ 前缀）工具片段；MUST 含一句引导：MCP/custom 工具以本会话定稿后的请求 tools 列表为准并按精确名调用（MAY 提示用户经产品 `/mcp` 查看连接与 armed）。MUST NOT 在 Available tools 段枚举 mcp 工具名。定稿后 provider tools 参数 MUST 可含当时冻结的 mcp 工具；MUST NOT 暗示 settle 后会继续热扩 tools 表。自定义 SYSTEM.md 或 custom_prompt 整段替换默认正文时 MUST 保持 pt9 替换语义。

  @req:pt12 @human
  场景: date-placement-and-session-env
    - 系统提示若写入日历日 MUST 经 ContextPolicy.date_placement：默认 Omit（system 无 Current date，亦无 CWD）；SystemAsToday / SystemPinnedAtSession 仅为消融/lab。产品路径 MUST 将日历日/CWD 以状态栏族特殊类型 session_env（Env CustomMessage，project_for_llm 投影为 user）在用户真实输入落盘之前持久化：首轮或相对上次 session_env 的日历日/CWD 有变时追加；同日同 cwd MUST NOT 重复追加。压缩裁剪或 overflow 同轮 reload 之后、继续向模型生成之前，上下文 MUST 仍含与当前日历日/进程 cwd 对齐的 session_env（缺失或日/cwd 过期则 ensure 追加并持久化）。秒级时间戳 MUST NOT 进系统提示。完整状态栏 Lane 由后继 change 消费既有 session_env。由单测覆盖；MUST NOT 为静态存在性单独扩 BDD step。

  @req:pt13 @human
  场景: no-instructions-dual-copy
    - openai-responses 主路径 MUST NOT 将系统提示再写入顶栏 instructions 与 input 前缀各一份；SSOT 为 system_prompt → input 前缀 item（thinking on 可为 developer）。由 Assembler/单测回归覆盖。

  @executable @req:pt3
  场景: no-slash-prompt-templates
    假如 项目或全局 prompts 目录存在 greet.md
    当 装配资源加载器并发现
    那么 get_commands MUST NOT 含 template:greet 或 /greet 模板命令且 loader MUST NOT 将 greet 注册为 prompt 模板

  @executable @req:pt9
  场景: collect-tool-guidelines
    假如 工具集含 bash 且其 prompt_guidelines 非空
    当 set_tools 或等价装配后 build_system_prompt
    那么 输出含 Guidelines 段且含该工具 guideline 短句

  @executable @req:pt9
  场景: custom-prompt-no-silent-tools-backfill
    假如 custom_prompt 或 SYSTEM.md 整段替换默认正文且未附 Available tools
    当 build_system_prompt
    那么 正文以该替换内容为主且 MUST NOT 偷偷回填默认 Available tools 清单
