# language: zh-CN
# purpose: 运行时配置 — 分层加载（global → project → --config）、深合并、secret.env、settings；配置节由插件声明（core 只做 loader + 注册表），引导段 xylitol，未知键两阶段校验，schema-first。仅 config.yaml + secret.env，不支持 config.local。

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
    配置值解析（resolve_from_env / resolve_value，环境变量、shell 命令与 ${VAR:-default} 解析，或等价）须收敛于 infra 配置边界且零内部依赖；agent 层 不得含重复解析实现。
    场景: 配置值解析器在基建边缘
      假如 仅存在 config.local.yaml 含可观测字段而无同层 config.yaml
      当 从全局 config.yaml 加载完整 settings
      那么 全局 config.yaml 生效且不经 config.local.yaml 合并

  @req:r1740
  规则: 配置边界承载 JsonSchema
    需要对外暴露 JSON Schema 的配置结构必须在配置边界定义或包装，不得为 schema 去改会话词汇。 由配置边界 JsonSchema 快照/单测覆盖，不单独扩 BDD step。

  @req:r1741
  规则: mcp_servers 配置字段
    运行时配置 须提供 mcp_servers（或等价）字段以声明 MCP 服务器；缺省或空表示未启用 MCP。

    场景: 字段
      当 读取默认配置
      那么 mcp 未启用且字段可序列化
  @req:r1727
  规则: model-entry-thinking-levels
    ModelsConfig 的 ModelEntry 须支持可选 thinking_level_map 作为用户面档位 SSOT：键为档名字符串（须非空），值为字符串或 null；声明键 须即该模型可展示、可 set 的支持集，按规范档位顺序（off、minimal、low、medium、high、xhigh、max）排列，非规范档名附后按字典序；空键 须使配置加载失败；不得仅因档名不属于历史封闭枚举而失败。缺省或空映射时运行时支持集 须仅为 off（见 m9）。Settings.default_thinking_level 若存在 MAY 仅用于会话首次装配的初始档（∈ 支持集则采用，否则忽略）；换模与精确 /model <id> 默认档 须遵循 m10 末项策略，不得被 Settings 默认覆盖。

    场景: 解析映射键为档位
      假如 YAML 模型条目 thinking_level_map 含 off: null 与 high: max 与 max: 32768
      当 加载配置并 resolve_model_meta
      那么 XyModelMeta.thinking_levels 与列表一致

    场景: 空键失败
      假如 thinking_level_map 含空键
      当 加载配置
      那么 失败

    场景: 自由文本成功
      假如 thinking_level_map 含厂商字面量 bogon-level: high
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
  @req:r1728
  规则: model-entry-thinking-level-map
    thinking_level_map 的值 须精确进入模型 meta 供请求组装：字符串值 须作为该档的 provider wire 值；null 值 须表示该档不向 provider 发送 thinking/effort 字段；未声明的档名 不得出现在支持集或可选列表；键与值的匹配 须精确字符串相等（不得因仅大小写或空白差异而匹配）。

    场景: 解析映射
      假如 YAML 含 thinking_level_map off: null 与 high: max
      当 加载并 resolve_model_meta
      那么 meta 含 high→max 与 off→null

    场景: 空串值失败
      假如 thinking_level_map 含空串值
      当 加载配置
      那么 失败

    场景: 空映射成功
      假如 thinking_level_map 为空映射
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
    Agent profile / AppConfig 不得暴露或生效 max_iterations（或等价已移除硬闸名）配置字段；schema 与加载路径 不得提供向后兼容 shim。可选 session.max_steps_per_turn（正整数）须允许且仅经 should_stop_after_step 装配生效（见 r1048）；缺省 须不限制步数；旧键 session.max_steps_per_turn 须使加载失败并指路新键。残留 max_iterations 键 不得再限制 ReAct 迭代。由配置单测或加载用例覆盖，不得为静态字段缺失单独扩 BDD step。
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
    SessionConfig 须支持可选 max_steps_per_turn（正整数）；缺省或省略 须为 None（不限制）；值为 0 或非法 须使配置加载失败；该字段 须供组合根安装 should_stop_after_step（ar30），不得映射为已移除的 max_iterations。由配置单测覆盖，不得为静态字段形状单独扩 BDD step。
    场景: 会话最大回合已加载
      假如 按 session.max_steps_per_turn=2 安装 should_stop_after_step 且入队 follow_up 以迫使第二轮
      当 运行 AgentRuntime
      那么 至多出现 2 次 StepStart 后出现 AgentEnd

  @req:r1739
  规则: tui-activity-fold
    AppConfig 须支持可选顶层 tui.activity_fold：enabled（缺省 true）、keep_recent_turns（缺省 2，非负整数）、stream_collapse（envelope 或 clusters，缺省 envelope）、auto_on_rebuild（缺省 true）、auto_on_turn_end（缺省 true）、display_level（quiet/summary/recent/full，缺省 summary）、visible_items（可选，含 thinking/tool_header/tool_output/diff）。非法 stream_collapse 或 display_level 须使配置加载失败。enabled 为 false 须使 ActivityFold 不套信封（全细账，块级折叠仍可用）。由配置单测覆盖，不得为静态字段形状单独扩 BDD step。
    场景: 活动折叠词法契约
      当 读取配置节字段缺省
      那么 活动折叠启用且保留两回合与信封折叠

  @req:r2928
  规则: schema-artifact-same-source
    配置的 JSON Schema 契约产物 须与运行时校验同一来源生成，不得手维护：产物内容 须等于由插件 section 注册表聚合导出的 schema 的稳定渲染（键序确定、可重复生成字节相同），且 须覆盖当前全部顶层配置节。产物与代码不一致时 门禁 须失败并指向再生成入口；产物 不得作为加载期校验来源（加载仍按结构导出即时校验）。

    场景: 产物与代码一致
      当 生成配置 schema 并与仓库产物比较
      那么 二者逐字节相同且再生成幂等

    场景: 产物覆盖当前顶层节
      当 读取仓库配置 schema 的顶层属性
      那么 含 models 与 agents 与 session 与 tool_batch 与 tui 与 otel 与 loop_policies 与 todo 与 workflow 与 tools 与 serve

  @req:r2959
  规则: feature-plugin-config-sections
    todo / workflow 须为顶层配置节。todo 缺省启用（progress / reminder / checkpoint 缺省停用）；workflow 缺省停用。旧 `loop_example` 配置节 须被拒绝，并提示迁移到 `loop_policies`；`loop_policies.todo_progress / todo_reminder / todo_reminder_stale_after / todo_checkpoint / workflow` 键 须被拒绝，并提示迁移到新节；顶层 `plan` 与 `loop_policies.plan_mode` 须被拒绝并提示 plan mode 已移除。默认值与旧键拒绝 由配置 crate 单测覆盖。

    场景: 顶层功能配置节
      假如 配置含 todo / workflow 节
      当 加载配置
      那么 各节按缺省启用状态生效且旧 loop_example 被拒绝并提示迁移

  @req:r2978
  规则: tools-selection-list
    配置根键 `tools` 须为字符串列表；每项 须为非空词元，可带 `+` / `-` 前缀。空串、仅前缀、空白词元 须使配置加载失败；未知工具名 不得在加载期失败（装配期告警，见 agent-tools tool-selection-list）。列表顺序 须保留，供装配期按序覆盖。

    场景: 解析工具列表
      假如 YAML 含 tools [read, -grep, +ffgrep]
      当 加载配置
      那么 tools 列表保序且词元一致

    场景: 工具列表空词元失败
      假如 YAML 含 tools 列表且含空字符串
      当 加载配置
      那么 失败

    场景: 未知工具名加载成功
      假如 YAML 含 tools [read, +no-such-tool]
      当 加载配置
      那么 成功且 tools 列表含 no-such-tool

  @req:r2987
  规则: legacy-provider-preset-migration
    预设回滚后，`models.entries.<alias>.preset` 与顶层 `provider_presets` 旧写法 须被加载器拒绝，并给出迁移指引（显式 `provider` / `model` / `base_url` / `api` / `compat`，或经网关）；不得静默忽略旧键使配置看似生效。正常配置 不得因该迁移检查失败。

    场景: 旧预设键拒绝并提示迁移
      假如 YAML 模型条目含 preset opencode-zen
      当 以加载器加载该配置
      那么 加载失败且错误含 preset 迁移指引

  @req:r2989
  规则: agent-profile-preset-fields
    `agents.profiles.<name>` 须支持 `persona` 与 `tools`：`persona` 与 `system_prompt` 均非空时，系统提示 须按 persona → 空行 → system_prompt 顺序拼接；`tools` 非空时 须覆盖全局 `tools` 选择列表，为空时 须沿用全局。`--agent-profile <name>` 须按名选择预设；未知预设 须硬失败并指向 `agents.profiles`，不得静默回退默认预设。

    场景: 预设 persona 与工具目录
      假如 YAML 含 agents.profiles.review 的 persona 与 tools
      当 以 --agent-profile review 解析装配
      那么 系统提示按 persona 在前拼接且工具选择为 profile tools

    场景: 未知预设硬失败
      假如 YAML 不含 agents.profiles.missing
      当 以 --agent-profile missing 解析装配
      那么 装配失败且错误指向 agents.profiles
  @req:r3016
  规则: model-entry-max-output-tokens
    配置 必须 接受 models.*.max_output_tokens 可选正整数并进入配置 schema 产物；声明 0 必须 被配置解析拒绝。该字段 不得 与 thinking 预算字段混用。
  @req:r3018
  规则: model-entry-prompt-cache-config
    配置 必须 接受 models.*.prompt_cache（`enabled` 可选布尔、`ttl` 可选 `5m` / `1h`、`retention` 可选 `in_memory` / `24h`）并进入配置 schema 产物；显式字段 必须 覆盖 compat 命名档提供的默认，不得 改变 compat 档自身的语义。
  @req:r3069
  规则: config-include-removed
    配置加载 不得 支持顶层 `include` 列表，也不得 提供通用 include / presets / import 机制；配置中出现 `include` 键 须 加载期硬失败并给出移除提示。由配置插件单测覆盖。
  @req:r3076
  规则: config-drop-in-fragments
    配置加载 须在每层 config.yaml 之后按文件名升序合并同层 `conf.d/*.yaml` 片段（global 层与 project 层各一次），再进入下一层与 `--config`；片段 须按普通配置片段解析（锚点、x- 前缀载体剥离、未知键硬失败同样适用），不得因此恢复通用 include / presets / import；片段解析或校验失败 须使加载失败且错误含片段文件路径。由配置插件单测覆盖。
  @req:r3064
  规则: serve-config
    运行时配置必须接受可选顶层 serve 块：lifecycle 为 last-client 或 persistent，idle_timeout 为时长或 false。缺省（不写该块）为 last-client 与约 30 秒。idle_timeout 为 false 时自动拉起必须按 persistent。该块仅约束自动拉起的 Host；显式 serve 缺省仍 persistent。由配置插件单测覆盖。

  @req:r3070
  规则: plugin-owned-config-sections
    所有顶层配置节 须由其所属插件声明（键名、schema、默认值）；核心 须仅承担加载、合并与注册表聚合，不得 维护手写字段清单。顶层未知键 须 两阶段校验：加载期先收集，插件装配后由插件按 rutis 依赖顺序认领自己的节，剩余未知键 须 硬失败；`xylitol.unknown_keys: warn` 时 须 降级为警告。节内未知键 须 由插件 schema 决定，默认拒绝；未知枚举值 须 硬失败。

    场景: 未知顶层键硬失败
      假如 配置含注册表未声明的顶层键
      当 加载并装配配置
      那么 硬失败并指出该键

    场景: 未知键放宽为警告
      假如 配置含未知顶层键且 xylitol.unknown_keys 为 warn
      当 加载并装配配置
      那么 装配成功且产生警告

  @req:r3071
  规则: bootstrap-section
    配置 须支持可选顶层 `xylitol` 引导段，含 `schema_version`、`unknown_keys`（error / warn，缺省 error）与 `plugins` 启用清单；`schema_version` 缺失时 须按当前版本处理，显式版本高于当前 须 硬失败。引导段 不得 承载插件专属字段。

    场景: 引导段缺省
      假如 配置不含 xylitol 引导段
      当 加载配置
      那么 按当前 schema 版本与 error 策略处理

  @req:r3072
  规则: yaml-merge-key
    配置 YAML 的 merge key `<<` 须在 `Value` 层展开：值为 mapping 或 mapping 列表时，父 mapping 的键 须 优先于合并来源，来源按列表顺序后者覆盖前者；值为非法类型 须 使加载失败；anchor / alias 原生语义 须 保持不变。

    场景: 合并键父键优先
      假如 YAML 含 base 锚点与 `<<: *base` 且父键覆盖同名字段
      当 加载配置
      那么 父键生效

    场景: 合并键非法值失败
      假如 YAML 的 `<<` 值为标量
      当 加载配置
      那么 失败

  @req:r3078
  规则: session-stage-name-config
    SessionConfig 须支持可选 `stage_name` 块：`enabled`（布尔，缺省 true）、`target_cjk_chars`（正整数，缺省 15）、`target_words`（正整数，缺省 20）、`max_title_bytes`（正整数，缺省 120）、`max_input_bytes`（正整数，缺省 8192）、`max_output_tokens`（正整数，缺省 64）、`timeout_ms`（正整数，缺省 8000）、可选 `provider` / `model`。非法值（0 / 负数 / 空 provider 或 model）须使配置加载失败；缺省块 须等价默认值。由配置单测覆盖，不扩 BDD step。

  @req:r3221
  规则: trust-audit-config
    config.yaml 须支持 trust 节：audit（布尔，缺省 true）、audit_max_bytes（正整数，缺省 10485760）、audit_keep_files（正整数，缺省 5）、audit_retention_days（正整数，缺省 90）；audit=false 时 不得写审计；非法值 须使配置校验失败。由配置单测覆盖，不扩 BDD step。

  @req:r3259
  规则: codemode-config
    config.yaml 须支持 codemode 节：enabled（缺省 false）、runtime=wasmtime、mode=on、timeout_ms、max_output_tokens、max_result_bytes、max_nested_calls、fuel、max_memory_bytes、module_cache、inline_budget、exposure。未配置零装配；非法 runtime/mode 须加载失败。由配置单测覆盖，不扩 BDD step。

  @req:r3311
  规则: context-inject-section
    配置 必须 支持 context_inject 节：injectors 数组，每项含 id / point / command / placement / dedup / timeout_secs / group；point 必须 限于七个注入点。三层（global → project → user）同 id 必须 后层覆盖；重复 id 与未知 point 必须 报错。project 层 必须 随 trust 门禁。由配置单测覆盖。

    场景: context_inject 重复 id 报错
      假如 配置含两条同 id 的 context_inject 注入器
      当 校验配置
      那么 校验报错
