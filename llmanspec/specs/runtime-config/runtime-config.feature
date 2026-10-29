# language: zh-CN
# purpose: 运行时配置 — 核心 config 插件分层加载（global → project → --config）、深合并、secret.env、settings；仅 config.yaml + secret.env，不支持 config.local。

功能: runtime-config

  @req:r1723
  规则: mode 字段
    Settings 须含 steering_mode 与 follow_up_mode，枚举含 all 与 one-at-a-time；缺省（未配置）须为 one-at-a-time；二者 须经组合根装配进插话/续跑队列模式。

    场景: 模式设置
      假如 settings.json 中 steering_mode 设为 one-at-a-time
      当 加载 settings
      那么 Settings.steering_mode 为 OneAtATime

    场景: 模式缺省一在时间
      假如 settings 未配置 steering_mode 与 follow_up_mode
      当 读取缺省
      那么 二者均为 OneAtATime
  @req:r1724
  规则: Settings 交付面
    Settings（settings.json）须仅含组合根已接线字段：default_thinking_level、compaction、thinking_budgets、steering_mode、follow_up_mode。未接线偏好（如 transport、shell_path、npm_command、session_dir、http_*、themes 路径列表、enable_skill_commands、markdown、warnings、default_provider/model）不得作为可生效 Settings 字段保留；遗留 JSON 键 须被忽略且 不得生效。

    场景: 设置投递表面仅
      假如 加载默认或示例 settings
      当 检查 Settings 类型与合并结果
      那么 Settings 仅含已接线交付字段
  @req:r1725
  规则: 无 prompts 列表
    Settings 不得再提供 prompts 路径列表字段（slash prompt 模板已移除）；遗留 prompts 键 须被忽略且 不得生效。

    场景: 无提示词设置字段
      假如 加载默认或示例 settings
      当 检查 Settings 类型与合并结果
      那么 不存在可生效的 prompts 路径列表字段
  @req:r1726
  规则: compaction 配置映射
    运行时配置 须定义从 YAML 加载期 compaction 表面（enabled / reserveTokens / keepRecentTokens / model / thinkingLevel）到规范运行时 CompactionSettings 的单一文档化映射，无并行重复类型；不得暴露或映射 compaction_threshold / 百分比阈值字段。model 为与 models 条目字段形状兼容的内嵌对象，跨条目共享 须靠 YAML 锚点/别名原生能力，不得引入字符串别名引用语义；model 与 thinkingLevel 仅 config.yaml 加载面生效，Settings.json 的 compaction 块 不得接线这两键（出现即忽略，对齐 rc12 遗留键纪律）。

    场景: 映射已文档化
      假如 config.yaml 含 compaction 节及 keepRecentTokens
      当 加载配置并解析为运行时 settings
      那么 compaction_settings.keep_recent_tokens 等于 YAML 中设置的值

    场景: 压缩模型锚点别名
      假如 config.yaml 的 compaction.model 经 YAML 别名引用与 models 条目同源的锚点
      当 加载配置
      那么 compaction 任务模型条目与该锚点条目字段一致

    场景: 压缩模型设置忽略
      假如 settings.json 的 compaction 块含 model 或 thinkingLevel 键
      当 加载 settings
      那么 两键被忽略且不进入运行时 CompactionSettings
  @req:r1722
  规则: ConfigValue 在 infra
    ConfigValueResolver（环境变量、shell 命令与 ${VAR:-default} 解析，或等价）须收敛于 infra 配置边界且零内部依赖；agent 层 不得含重复解析实现。
    场景: 配置值解析器在基建边缘
      假如 仅存在 config.local.yaml 含可观测字段而无同层 config.yaml
      当 从全局 config.yaml 加载完整 settings
      那么 全局 config.yaml 生效且不经 config.local.yaml 合并

  @req:r1740
  规则: 配置边界承载 JsonSchema
    需要对外暴露 JSON Schema 的配置结构必须在配置边界定义或包装，不得为 schema 去改会话词汇。

  @req:r1741
  规则: mcp_servers 配置字段
    运行时配置 须提供 mcp_servers（或等价）字段以声明 MCP 服务器；缺省或空表示未启用 MCP。

    场景: 字段
      当 读取默认配置
      那么 mcp 未启用且字段可序列化
  @req:r1727
  规则: model-entry-thinking-levels
    ModelsConfig 的 ModelEntry（或等价 YAML 面）须支持可选 thinking_levels 有序字符串列表作为用户面档位 SSOT；空串或仅空白条目 须使配置加载失败；不得仅因档名不属于历史封闭枚举而失败。缺省或未声明可调列表时运行时支持集 须仅为 off（见 m9）。Settings.default_thinking_level 若存在 MAY 仅用于会话首次装配的初始档（∈ 支持集则采用，否则忽略）；换模与精确 /model <id> 默认档 须遵循 m10 末项策略，不得被 Settings 默认覆盖。

    场景: 解析列表
      假如 YAML 模型条目含 thinking_levels [off, high, max]
      当 加载配置并 resolve_model_meta
      那么 XyModelMeta.thinking_levels 与列表一致

    场景: 空词元失败
      假如 thinking_levels 含空字符串
      当 加载配置
      那么 失败

    场景: 自由文本成功
      假如 thinking_levels 含厂商字面量 bogon-level
      当 加载配置
      那么 成功且支持集含 bogon-level

    场景: 缺省设置
      假如 Settings.default_thinking_level 为 high 且模型支持集为 off 与 high 与 max
      当 会话首次装配
      那么 当前 thinking level 为 high

    场景: 选择忽略设置缺省
      假如 Settings.default_thinking_level 为 off 且模型支持集为 off 与 high 与 max
      当 select_model 到该模型
      那么 thinking level 为 max
    # （c2827 合并：runtime-model-registry r1758 thinking-levels-on-meta 的 meta 解析语义由本条 parse-list/freeform-ok 场景承载。）
  @req:r2976
  规则: model-entry-presets
    ModelsConfig 的 ModelEntry 须支持可选 `preset` 字符串；预设目录 须为编译期常量，至少含 zmodel / opencode-zen / clinepass / commandcode。加载期 须先展开预设默认值（provider / model / base_url / api / compat / thinking / thinking_levels / context_window），再用条目显式字段覆盖；预设 不得提供 api_key / fallback / system prompt，也不得改变工具或 ReAct 语义。未知 preset 须使配置加载失败并给出 `unknown preset: <id>` 与模型别名，不得静默回落裸档案；`context_window` 为 0 的预设 须留给 provider 默认，不得伪造。预设名 须为唯一短名，不得为旧名保留别名。

    场景: 预设展开
      假如 YAML 模型条目仅含 preset: opencode-zen 与 api_key
      当 加载预设配置
      那么 provider 为 openai 且 model 与 base_url 与 api 来自预设

    场景: 显式覆盖
      假如 YAML 模型条目含 preset: zmodel 且显式 model 与 context_window
      当 加载预设配置
      那么 显式字段覆盖预设默认值

    场景: 未知预设失败
      假如 YAML 模型条目含 preset: no-such
      当 加载预设配置
      那么 失败且诊断含 unknown preset

  @req:r1728
  规则: model-entry-thinking-level-map
    ModelsConfig 的 ModelEntry 须支持可选 thinking_level_map（键为档名字符串，值为字符串或 null）；键不在该模型 thinking_levels 声明列表中（缺省列表视为仅 off）时 须使配置加载失败；键与声明列表的归属判定 须精确字符串相等（不得因仅大小写或空白差异而匹配）；解析结果 须进入模型 meta 供请求组装；键缺省 须表示使用 adapter 内置默认，null 须表示该档不向 provider 发送 thinking/effort 字段。

    场景: 解析映射
      假如 YAML 含 thinking_levels [off, high] 与 thinking_level_map high: max 与 off: null
      当 加载并 resolve_model_meta
      那么 meta 含 high→max 与 off→null

    场景: 映射键之外列表失败
      假如 thinking_levels 为 [off, high] 且 thinking_level_map 含 max: high
      当 加载配置
      那么 失败

    场景: 缺席键成功
      假如 仅配置 thinking_levels 无 map
      当 加载配置
      那么 成功且 map 为空或缺省
  @req:r1731
  规则: config-yaml-secret-env-only
    产品配置 须仅以 config.yaml（可分享非密钥）与 secret.env（密钥；YAML 经 {{ secret.KEY }}）为配置面；加载链 须为 global config.yaml → project config.yaml → 可选 --config，外加 secret.env 注入；AppConfig 全局目录 须以 ~/.config/xylitol/ 为 SSOT，~/.xylitol/ 须定位为数据目录（sessions/logs 等），不得鼓励在 ~/.xylitol/config.yaml 放置 AppConfig（migrate 兼容除外）。不得再把本地词表缓存目录作为产品数据面承诺。

    场景: 配置YAML密钥环境变量布局
      假如 配置加载器已就绪
      当 从全局 config.yaml 加载完整 settings
      那么 全局 config.yaml 生效且不经 config.local.yaml 合并
  @req:r1732
  规则: config-local-unsupported
    加载器 不得读取或深合并 config.local.yaml 或 config.local.yml（global 与 project）；磁盘上若存在这些文件 须忽略其内容且 不得提供自动迁移到 config.yaml/secret.env；MAY 经加载诊断提示已忽略。

    场景: 配置本地不已合并
      假如 仅存在 config.local.yaml 含可观测字段而无同层 config.yaml
      当 加载配置
      那么 该字段不生效（local 被忽略）
  @req:r1733
  规则: no-agent-max-iterations
    Agent profile / AppConfig 不得暴露或生效 max_iterations（或等价已移除硬闸名）配置字段；schema 与加载路径 不得提供向后兼容 shim。可选 session.max_turns（正整数）须允许且仅经 should_stop_after_turn 装配生效（见 ar30）；缺省 须不限制轮次。残留 max_iterations 键 不得再限制 ReAct 迭代。由配置单测或加载用例覆盖，不得为静态字段缺失单独扩 BDD step。
    场景: 无最大轮次字段
      假如 加载默认或示例 settings
      当 检查 Settings 类型与合并结果
      那么 Settings 仅含已接线交付字段

  @req:r1734
  规则: config-yaml-vars-home
    配置 YAML minijinja 模板 须提供命名空间 vars，且 须仅暴露键 home（解析为用户 home 目录绝对路径）；{{ vars.home }} 须可在任意配置字符串字段插值；home 不可解析或引用 vars 下其它键 须使模板渲染以 strict 错误失败；不得在 vars 下提供 project、cwd 或其它路径键。由配置模板单测覆盖，不得为该插值单独扩 BDD step。
    场景: 模板变量家目录仅
      当 读取模板 vars 命名空间
      那么 vars 仅暴露 home

  @req:r1735
  规则: otel-config-section
    AppConfig 须支持可选顶层 otel（或等价）节：至少含 exporter（none 或 otlp-http；缺省等价 none）、可选 endpoint、protocol（http-binary 或 http-json）、environment、service_name 与 headers/凭证引用字段；非法 exporter/protocol 须使配置加载失败；该节 须仅控制远程 OTLP 出口，不得取代本地 file log / provider-trace 的环境与构建闸（见 infra-logging / infra-provider-trace）。由配置单测覆盖，不得为静态字段形状单独扩 BDD step。
    场景: OTLP节可选
      当 读取配置节字段缺省
      那么 otel 节可缺省且等价 none

  @req:r1736
  规则: tui-editor-history-seed-sessions
    AppConfig 须支持可选顶层 tui 节字段 editor_history_seed_sessions（非负整数）；缺省 须为 1；该值 须供产品 TUI 纯 new session 装载跨 session 发送历史种子（ati41）；由配置单测覆盖，不得为静态字段形状单独扩 BDD step。
    场景: 编辑器历史种子缺省一
      当 读取配置节字段缺省
      那么 tui 历史种子缺省为一

  @req:r1737
  规则: tool-batch-config
    AppConfig 须支持可选工具批配置节 tool_batch：含 mode（sequential 或 barrier_parallel；缺省 barrier_parallel）；非法 mode 须使配置加载失败；显式 sequential 须恢复源序串行批；不得提供将 mcp_ 工具升为并行安全的配置字段（无 parallel_safe_patterns 或等价 MCP 放行名单）。由配置单测覆盖，不得为静态字段形状单独扩 BDD step。
    场景: 工具批模式缺省与无效
      当 读取配置节字段缺省
      那么 工具批缺省并行且回合上限须为正整数

  @req:r1738
  规则: session-max-turns
    SessionConfig 须支持可选 max_turns（正整数）；缺省或省略 须为 None（不限制）；值为 0 或非法 须使配置加载失败；该字段 须供组合根安装 should_stop_after_turn（ar30），不得映射为已移除的 max_iterations。由配置单测覆盖，不得为静态字段形状单独扩 BDD step。
    场景: 会话最大回合已加载
      假如 按 session.max_turns=2 安装 should_stop_after_turn 且入队 follow_up 以迫使第二轮
      当 运行 AgentRuntime
      那么 至多出现 2 次 TurnStart 后出现 AgentEnd

  @req:r1739
  规则: tui-activity-fold
    AppConfig 须支持可选顶层 tui.activity_fold：enabled（缺省 true）、keep_recent_turns（缺省 2，非负整数）、stream_collapse（envelope 或 clusters，缺省 envelope）、auto_on_rebuild（缺省 true）、auto_on_turn_end（缺省 true）。非法 stream_collapse 须使配置加载失败。enabled 为 false 须使 ActivityFold 不套信封（全细账，块级折叠仍可用）。由配置单测覆盖，不得为静态字段形状单独扩 BDD step。
    场景: 活动折叠词法契约
      当 读取配置节字段缺省
      那么 活动折叠启用且保留两回合与信封折叠

  @req:r2928
  规则: schema-artifact-same-source
    配置的 JSON Schema 契约产物 须与运行时校验同一来源生成，不得手维护：产物内容 须等于由 AppConfig 结构导出的 schema 的稳定渲染（键序确定、可重复生成字节相同），且 须覆盖当前全部顶层配置节。产物与代码不一致时 门禁 须失败并指向再生成入口；产物 不得作为加载期校验来源（加载仍按结构导出即时校验）。

    场景: 产物与代码一致
      当 生成配置 schema 并与仓库产物比较
      那么 二者逐字节相同且再生成幂等

    场景: 产物覆盖当前顶层节
      当 读取仓库配置 schema 的顶层属性
      那么 含 models 与 agents 与 session 与 tool_batch 与 tui 与 otel 与 loop_policies 与 todo 与 workflow 与 plan

  @req:r2959
  规则: feature-plugin-config-sections
    todo / workflow / plan 须为顶层配置节。todo 缺省启用（progress / reminder / checkpoint 缺省停用）；workflow 缺省停用；plan 缺省启用且 read_only 缺省 true。旧 `loop_example` 配置节 须被拒绝，并提示迁移到 `loop_policies`；`loop_policies.todo_progress / todo_reminder / todo_reminder_stale_after / todo_checkpoint / plan_mode / workflow` 键 须被拒绝，并提示迁移到新节。默认值与旧键拒绝 由配置 crate 单测覆盖。
