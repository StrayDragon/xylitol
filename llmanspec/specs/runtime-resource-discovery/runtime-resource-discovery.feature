# language: zh-CN
# managed by llman sdd partition-migrate
功能: runtime-resource-discovery

  @req:rd1
  场景: list
    假如 ResourceLoader 已加载 skills 与 prompts
    当 调用 list
    那么 输出展示每项资源的 scope 与 path

  @req:rd1
  场景: list-empty
    假如 未安装任何资源
    当 调用 list
    那么 输出为空且退出码为零

  @req:rd2
  场景: info-found
    假如 存在名为 foo 的 skill
    当 调用 info foo
    那么 展示 foo 详情且退出为零

  @req:rd2
  场景: info-missing
    假如 不存在名为 bar 的资源
    当 调用 info bar
    那么 展示 not-found 消息且退出非零

  @req:rd3
  场景: doctor-clean
    假如 全部资源有效
    当 调用 doctor
    那么 无问题报告且退出为零

  @req:rd3
  场景: doctor-issues
    假如 存在缺少 frontmatter 的 SKILL.md
    当 调用 doctor
    那么 列出问题且退出非零

  @req:rd4
  场景: read-only
    假如 调用任意资源命令
    当 命令完成
    那么 无文件被创建或修改

  @req:rd5
  场景: reuse
    假如 ResourceLoader 已发现资源
    当 调用 list
    那么 展示相同集合且无独立重扫

  @req:rd6
  场景: create
    假如 提供资源元数据
    当 调用 create_source_info
    那么 返回字段完整的 SourceInfo

  @req:rd7
  场景: factory-create
    假如 提供含 path 与 scope 的资源元数据
    当 调用 create_source_info
    那么 返回完整填充的 SourceInfo

  @req:rd8
  场景: migration-skills
    假如 从项目目录加载 skill
    当 创建其 SourceInfo
    那么 scope 为 project

  @req:rd8
  场景: migration-commands
    假如 注册 slash 命令
    当 设置其 source_info
    那么 非来自包时 origin 为 TopLevel

  @req:rd9
  场景: scope-variants
    假如 从 user scope 加载资源
    当 创建其 SourceInfo
    那么 scope 为 User

  @req:rd10
  场景: reload-agents
    假如 loader 已加载且盘上 AGENTS.md 已改
    当 调用 reload
    那么 get_agents_files 反映新内容

  @req:rd10
  场景: reloadable-trait
    假如 runtime_protocol 导出 XyReloadable
    当 检查 DefaultResourceLoader
    那么 实现 XyReloadable 且 Outcome 可丢弃或含诊断

  @req:rd11
  场景: list-trusted
    假如 信任且项目 themes/foo.json 存在
    当 经 Trust 语义的 resource loader get_themes（项目 cwd）
    那么 列表含 foo

  @req:rd11
  场景: list-untrusted-themes
    假如 未信任且仅项目有 themes/secret.json
    当 经未信任语义的 loader（cwd 不指向项目）get_themes
    那么 列表 MUST NOT 含 secret

  @req:rd12
  场景: list-trusted
    假如 信任且项目 skills 含 demo
    当 get_skills 或 reload_skills
    那么 列表含 demo 且 scope 为 project

  @req:rd12
  场景: list-untrusted
    假如 未信任且仅项目有 skill
    当 get_skills 或 reload_skills（未信任）
    那么 列表 MUST NOT 含该项目 skill

  @req:rd12
  场景: reload-names
    假如 loader 已加载后盘上新增 skill
    当 调用 reload_skills
    那么 report names 含新 skill
