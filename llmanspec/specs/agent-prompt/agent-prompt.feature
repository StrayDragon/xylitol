# language: zh-CN
# managed by llman sdd partition-migrate
功能: agent-prompt

  @req:pt3
  场景: no-slash-prompt-templates
    假如 项目或全局 prompts 目录存在 greet.md
    当 装配资源加载器并发现
    那么 get_commands MUST NOT 含 template:greet 或 /greet 模板命令且 loader MUST NOT 将 greet 注册为 prompt 模板

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
