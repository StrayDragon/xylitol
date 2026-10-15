# language: zh-CN
# purpose: 系统提示组装：skills/context 注入与热应用；默认正文经沙箱 minijinja；产品主路径用 skill（含 $name），不得提供 pi 式 slash prompt templates。

功能: agent-prompt

  @req:r1015
  规则: system-assembly
    System 须经 build_system_prompt（或等价）组装系统提示，可包含工具片段、skills 摘要、context 文件；自定义 system 文件可替换或 append。组装顺序 须可文档化为 stable 正文与资源 → skills 元数据 → Guidelines → runtime_policy。产品默认 不得将日历日或 CWD 写入系统提示前缀（session_env 另条）；秒级时刻等 volatile 读数 不得进入系统提示前缀。
    场景: assembly-includes-tool-guidelines
      假如 工具集含 bash 且其 prompt_guidelines 非空
      当 set_tools 或等价装配后 build_system_prompt
      那么 输出含 Guidelines 段且含该工具 guideline 短句

  @req:r1020
  规则: skills-section
    当 ResourceLoader 发现 skills 时，系统提示 须能包含 available_skills（或等价）清单供模型发现；未信任项目 不得注入项目 skills。skills 元数据属 stable（会话作用域）；SKILL.md 正文仍经 $skill 注入 user 投影，不得为对齐外部书语而强制改走状态栏。
    场景: skills-listed-after-trust
      假如 Agent 能力聚合体已就绪
      当 热应用技能目录 greet
      那么 系统提示含 available_skills 清单

  @req:r1021
  规则: no-slash-prompt-templates·pt3
    System 不得从 prompts 目录发现或注册 slash prompt 模板命令（含 /template:name 与 pi 式 /filename）；磁盘遗留 prompts/*.md 须忽略；不得提供 $1/$@ 位置参数模板展开。产品主路径用 skills。

    场景: no-slash-prompt-templates
      假如 项目或全局 prompts 目录存在 greet.md
      当 装配资源加载器并发现
      那么 get_commands 不含 template:greet 或 /greet 模板命令且 loader 不将 greet 注册为 prompt 模板
  @req:r1022
  规则: safe-minijinja-default-assembly
    默认 system 组装路径（无 SYSTEM.md/custom_prompt 整段替换时）须经沙箱 minijinja 渲染嵌入默认模板：UndefinedBehavior::Strict；上下文仅白名单键（至少含 date、cwd、tools、guidelines、skills、runtime_policy、mcp_discover 或等价）；不得注册任意文件系统 loader；不得将 env/secret 命名空间注入该上下文。{% include %} 须仅解析预注册模板名。SystemPromptOpts（或等价）须允许注入 date（或等价时钟）。由单测覆盖安全与可钉 date；不得为静态依赖存在性单独扩重型 BDD（可保留窄正向场景或 feature:false）。
    场景: replacement-skip-default-fragments
      假如 custom_prompt 或 SYSTEM.md 整段替换默认正文且未附 Available tools
      当 build_system_prompt
      那么 正文以该替换内容为主且 不偷偷回填默认 Available tools 清单

  @req:r1023
  规则: context 热应用
    Agent 须提供 apply_prompt_resources（或等价）：用新的 context_files、system_prompt、append_system_prompt 重建系统提示；须只影响后续 run；不得改写已持久化历史消息。

    场景: apply-prompt-resources-next-run
      假如 Agent 能力聚合体已就绪
      当 热应用新的上下文与系统提示资源
      那么 系统提示按新资源重建
  @req:r1024
  规则: skills 热应用与引导注入
    Agent 须在构造/bootstrap 时将 ResourceLoader 发现的 skills 写入 SystemPromptOpts 并经 build_system_prompt 进入 available_skills（若非空）；须提供 apply_skills（或扩展 apply_prompt_resources）用新目录重建系统提示；须只影响后续 run；不得改写已持久化历史；未信任 不得注入项目 skills。

    场景: apply-skills-rebuilds-catalog
      假如 Agent 能力聚合体已就绪
      当 热应用技能目录 greet
      那么 系统提示含 available_skills 清单
  @req:r1025
  规则: dollar-skill 提交注入
    当用户消息含 $name 且 name 在已加载 skills 目录中时，System 须在发往模型前将对应 SKILL.md 正文（去 frontmatter）注入该 user 消息的模型侧投影；会话历史 须保留原始含 $ 文本；未知 $name 须透传；未信任 不得注入项目 skill 正文（目录已空即可）。

    场景: dollar-skill-expands-body
      假如 已加载技能 greet 正文为 "SKILL BODY"
      当 对含技能引用的输入展开技能
      那么 投影注入技能正文且未知名透传
  @req:r1026
  规则: tool-prompt-guidelines-assembly
    set_tools 或等价初次装配工具集时，System 须收集各 XyTool::prompt_guidelines（非空）写入 SystemPromptOpts.prompt_guidelines，并经 build_system_prompt 进入 Guidelines 段；内置 bash/read/edit/write（及已启用的同族工具）须提供与 pi 对齐的非空 guideline 短句。不得为抑制 XML 伪工具而注入防呆文案。自定义 SYSTEM.md 或 custom_prompt 整段替换默认正文时 须保持替换语义（不偷偷回填 Available tools）。

    场景: collect-tool-guidelines
      假如 工具集含 bash 且其 prompt_guidelines 非空
      当 set_tools 或等价装配后 build_system_prompt
      那么 输出含 Guidelines 段且含该工具 guideline 短句

    场景: custom-prompt-no-silent-tools-backfill
      假如 custom_prompt 或 SYSTEM.md 整段替换默认正文且未附 Available tools
      当 build_system_prompt
      那么 正文以该替换内容为主且 不偷偷回填默认 Available tools 清单
  @req:r1016
  规则: runtime-policy-fragments
    System 须按当前工具批模式（XyBatchMode）从内置表解析 runtime policy 片段并经 build_system_prompt 注入 <runtime_policy> 段（位于 APPEND_SYSTEM / Guidelines 之后；产品默认其后不再追加 date/cwd）；barrier_parallel 须注入多-tool 同消息策略片段；sequential 不得注入该片段；set_tool_mode（或等价）须在下一轮重建前提示中反映片段启停。Session 须持有已应用 fragment id 集合：同一 id 集合 不得重复同步/重写片段正文（同模式重复 set_tool_mode 不得导致 <runtime_policy> 重复段或同 body 多份）；模式变更后 id 集合变化时 须再同步一次。不得要求用户手写 APPEND_SYSTEM 才能获得该策略。由单测覆盖，不得为静态存在性单独扩 BDD step。
    场景: runtime-policy-fragments-injected
      假如 工具集含 bash 且其 prompt_guidelines 非空
      当 set_tools 或等价装配后 build_system_prompt
      那么 输出含 Guidelines 段且含该工具 guideline 短句

  @req:r1017
  规则: builtins-available-tools-mcp-discover
    默认 build_system_prompt 路径下 Available tools 散文清单 须仅含内置（非 mcp__ / 过渡 mcp- / mcp_ 前缀）工具片段；须含一句引导：MCP/custom 工具以本会话定稿后的请求 tools 列表为准并按精确名调用（MAY 提示用户经产品 `/mcp` 查看连接与 armed）。不得在 Available tools 段枚举 mcp 工具名。定稿后 provider tools 参数 须可含当时冻结的 mcp 工具；不得暗示 settle 后会继续热扩 tools 表。自定义 SYSTEM.md 或 custom_prompt 整段替换默认正文时 须保持 pt9 替换语义。

    场景: available-tools-omits-mcp-names
      假如 工具片段含 mcp 前缀工具
      当 以默认路径组装系统提示
      那么 系统提示不含 mcp 工具名且含 MCP 引导句
  @req:r1018
  规则: system-omit-date-and-session-env
    系统提示 不得写入日历日或 CWD（无 date_placement 旋钮，日历日/CWD 由 session_env 提供）。产品路径 须将日历日/CWD 以状态栏族特殊类型 session_env（Env CustomMessage，project_for_llm 投影为 user）在用户真实输入落盘之前持久化：首轮或相对上次 session_env 的日历日/CWD 有变时追加；同日同 cwd 不得重复追加。压缩裁剪或 overflow 同轮 reload 之后、继续向模型生成之前，上下文 须仍含与当前日历日/进程 cwd 对齐的 session_env（缺失或日/cwd 过期则 ensure 追加并持久化）。秒级时间戳 不得进系统提示。完整状态栏 Lane 由后继 change 消费既有 session_env。由单测覆盖；不得为静态存在性单独扩 BDD step。
    场景: date-and-cwd-from-session-env
      当 读取默认系统提示模板与装配
      那么 模板只带工具与 mcp 而日期与 cwd 由 session_env 补齐

  @req:r1019
  规则: no-instructions-dual-copy
    openai-responses 主路径 不得将系统提示再写入顶栏 instructions 与 input 前缀各一份；SSOT 为 system_prompt → input 前缀 item（thinking on 可为 developer）。由 Assembler/单测回归覆盖。
    场景: system-prompt-single-copy
      假如 Responses 组装且 system_prompt 非空且 thinking_level 为 medium
      当 转换为 input items
      那么 首项 role 为 developer 且 content 为 system_prompt
