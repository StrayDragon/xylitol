# language: zh-CN
# managed by llman sdd partition-migrate
功能: agent-prompt

  @req:pt1
  场景: assemble
    假如 提供 tools 与 context
    当 build_system_prompt
    那么 输出含工具与 context 片段

  @req:pt2
  场景: skills
    假如 已加载 demo-skill
    当 组装系统提示
    那么 含 skill 名或 available_skills 段

  @req:pt2
  场景: skills-untrusted
    假如 项目未信任且仅有项目 skill
    当 组装
    那么 MUST NOT 含该项目 skill

  @req:pt3
  场景: no-slash-prompt-templates
    假如 项目或全局 prompts 目录存在 greet.md
    当 装配 AgentSession 或 ResourceLoader 发现
    那么 get_commands MUST NOT 含 template:greet 或 /greet 模板命令且 loader MUST NOT 将 greet 注册为 prompt 模板

  @req:pt5
  场景: no-jinja-dep
    当 检查 Cargo.toml 与 src/agent/prompt
    那么 无 jinja/minijinja 依赖

  @req:pt6
  场景: apply-next-turn
    假如 会话已有 user/assistant 历史
    当 apply_prompt_resources 换新 AGENTS 内容
    那么 历史条数不变且下一轮 system 含新 context

  @req:pt7
  场景: bootstrap-inject
    假如 信任且已加载 demo-skill
    当 构造 Agent 或 bootstrap 装配
    那么 系统提示含 demo-skill 或 available_skills 段

  @req:pt7
  场景: untrusted-skip
    假如 未信任且仅有项目 skill
    当 装配系统提示
    那么 MUST NOT 含该项目 skill 名

  @req:pt7
  场景: apply-keeps-history
    假如 会话已有历史
    当 apply_skills 换新目录
    那么 历史条数不变且下一轮 system 反映新 skills

  @req:pt8
  场景: inject-known
    假如 目录含 demo 且用户文本含 $demo
    当 展开后发模型
    那么 模型侧 user 文本含 SKILL.md 正文片段

  @req:pt8
  场景: history-raw
    假如 同上
    当 检查会话历史 user 文本
    那么 仍含字面 $demo 且无强制展开块作为唯一形态

  @req:pt8
  场景: unknown-passthrough
    假如 文本含 $nosuch
    当 展开
    那么 无 nosuch 的 skill 块；$nosuch 仍在文本中

  @req:pt9
  场景: collect-tool-guidelines
    假如 工具集含 bash 且其 prompt_guidelines 非空
    当 set_tools 或等价装配后 build_system_prompt
    那么 输出含 Guidelines 段且含该工具 guideline 短句

  @req:pt9
  场景: custom-prompt-no-silent-tools-backfill
    假如 custom_prompt 或 SYSTEM.md 整段替换默认正文且未附 Available tools
    当 build_system_prompt
    那么 正文以该替换内容为主且 MUST NOT 偷偷回填默认 Available tools 清单

  @req:pt11
  场景: builtins-only-available-tools-mcp-discover
    假如 ToolSet 含 builtins 与至少一个 mcp: 工具且使用默认正文
    当 build_system_prompt
    那么 Available tools 段无 mcp: 工具名且含按本轮 tools 列表发现并精确调用的引导句
