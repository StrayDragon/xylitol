# language: zh-CN
# capability: runtime-config
# purpose: 运行时配置 — 深合并、secret.env、settings；仅 config.yaml + secret.env，不支持 config.local。
# scope: src/infra/config/, src/infra/settings/

功能: runtime-config

  @req:rc12 @human
  场景: mode 字段
    - Settings MUST 含 steering_mode 与 follow_up_mode，枚举含 all 与 one-at-a-time；缺省（未配置）MUST 为 one-at-a-time；二者 MUST 经组合根装配进插话/续跑队列模式。

  @req:rc13 @human
  场景: Settings 交付面
    - Settings（settings.json）MUST 仅含组合根已接线字段：default_thinking_level、compaction、thinking_budgets、steering_mode、follow_up_mode。未接线偏好（如 transport、shell_path、npm_command、session_dir、http_*、themes 路径列表、enable_skill_commands、markdown、warnings、default_provider/model）MUST NOT 作为可生效 Settings 字段保留；遗留 JSON 键 MUST 被忽略且 MUST NOT 生效。

  @req:rc14 @human
  场景: 无 prompts 列表
    - Settings MUST NOT 再提供 prompts 路径列表字段（slash prompt 模板已移除）；遗留 prompts 键 MUST 被忽略且 MUST NOT 生效。

  @req:rc15 @human
  场景: compaction 配置映射
    - 运行时配置 MUST 定义从 YAML 加载期 compaction 表面（enabled / reserveTokens / keepRecentTokens / model / thinkingLevel）到规范运行时 CompactionSettings 的单一文档化映射，无并行重复类型；MUST NOT 暴露或映射 compaction_threshold / 百分比阈值字段。model 为与 models 条目字段形状兼容的内嵌对象，跨条目共享 MUST 靠 YAML 锚点/别名原生能力，MUST NOT 引入字符串别名引用语义；model 与 thinkingLevel 仅 config.yaml 加载面生效，Settings.json 的 compaction 块 MUST NOT 接线这两键（出现即忽略，对齐 rc12 遗留键纪律）。

  @req:rc1 @human
  场景: ConfigValue 在 infra
    - ConfigValueResolver（环境变量、shell 命令与 ${VAR:-default} 解析，或等价）MUST 收敛于 infra 配置边界且零内部依赖；agent 层 MUST NOT 含重复解析实现。

  @req:rc8 @human
  场景: 配置边界承载 JsonSchema
    - 需要对外暴露 JSON Schema 的配置结构 MUST 在 runtime/config 边界（infra/config 或 settings）定义或包装，并 MUST 能映射到 domain 纯数据（若存在对应领域类型）。

  @req:rc9 @human
  场景: mcp_servers 配置字段
    - 运行时配置 MUST 提供 mcp_servers（或等价）字段以声明 MCP 服务器；缺省或空表示未启用 MCP。

  @req:rc16 @human
  场景: model-entry-thinking-levels
    - ModelsConfig 的 ModelEntry（或等价 YAML 面）MUST 支持可选 thinking_levels 有序字符串列表作为用户面档位 SSOT；空串或仅空白条目 MUST 使配置加载失败；MUST NOT 仅因档名不属于历史封闭枚举而失败。缺省或未声明可调列表时运行时支持集 MUST 仅为 off（见 m9）。Settings.default_thinking_level 若存在 MAY 仅用于会话首次装配的初始档（∈ 支持集则采用，否则忽略）；换模与精确 /model <id> 默认档 MUST 遵循 m10 末项策略，MUST NOT 被 Settings 默认覆盖。

  @req:rc17 @human
  场景: model-entry-thinking-level-map
    - ModelsConfig 的 ModelEntry MUST 支持可选 thinking_level_map（键为档名字符串，值为字符串或 null）；键不在该模型 thinking_levels 声明列表中（缺省列表视为仅 off）时 MUST 使配置加载失败；键与声明列表的归属判定 MUST 精确字符串相等（MUST NOT 因仅大小写或空白差异而匹配）；解析结果 MUST 进入模型 meta 供请求组装；键缺省 MUST 表示使用 adapter 内置默认，null MUST 表示该档不向 provider 发送 thinking/effort 字段。

  @req:rc18 @human
  场景: model-entry-tokenizer
    - ModelsConfig 的 ModelEntry MUST 支持可选字符串字段 tokenizer：可为顶层 tokenizers 表中的名字、HF owner/repo、本地路径或 builtin；AppConfig MUST 支持可选顶层 tokenizers 映射（条目含 repo/file 或 path）供多模型共用同一词表源；缺省 tokenizer 时 MUST 回退 registry builtin 启发式或视为未映射，MUST NOT 用推理用 model id 静默假定 HF 仓库。

  @req:rc19 @human
  场景: token-estimate-local-tokenizer
    - AppConfig MUST 支持可选 token_estimate.local_tokenizer，取值仅 on 或 off（或缺省等价 off）；默认 MUST 为 off；非法值 MUST 使配置加载失败；该闸 MUST 接到估计路径的 LocalTokenizer 允许位（paa10），MUST NOT 引入其它本地计数策略枚举。

  @req:rc20 @human
  场景: config-yaml-secret-env-only
    - 产品配置 MUST 仅以 config.yaml（可分享非密钥）与 secret.env（密钥；YAML 经 {{ secret.KEY }}）为配置面；加载链 MUST 为 global config.yaml → project config.yaml → 可选 --config，外加 secret.env 注入；AppConfig 全局目录 MUST 以 ~/.config/xylitol/ 为 SSOT，~/.xylitol/ MUST 定位为数据目录（sessions/tokenizers/logs 等），MUST NOT 鼓励在 ~/.xylitol/config.yaml 放置 AppConfig（migrate 兼容除外）。

  @req:rc21 @human
  场景: config-local-unsupported
    - 加载器 MUST NOT 读取或深合并 config.local.yaml 或 config.local.yml（global 与 project）；磁盘上若存在这些文件 MUST 忽略其内容且 MUST NOT 提供自动迁移到 config.yaml/secret.env；MAY 经 resources doctor 或加载诊断提示已忽略。

  @req:rc22 @human
  场景: no-agent-max-iterations
    - Agent profile / AppConfig MUST NOT 暴露或生效 max_iterations（或等价已移除硬闸名）配置字段；schema 与加载路径 MUST NOT 提供向后兼容 shim。可选 session.max_turns（正整数）MUST 允许且仅经 should_stop_after_turn 装配生效（见 ar30）；缺省 MUST 不限制轮次。残留 max_iterations 键 MUST NOT 再限制 ReAct 迭代。由配置单测或加载用例覆盖，MUST NOT 为静态字段缺失单独扩 BDD step。

  @req:rc23 @human
  场景: config-yaml-vars-home
    - 配置 YAML minijinja 模板 MUST 提供命名空间 vars，且 MUST 仅暴露键 home（解析为用户 home 目录绝对路径）；{{ vars.home }} MUST 可在任意配置字符串字段插值；home 不可解析或引用 vars 下其它键 MUST 使模板渲染以 strict 错误失败；MUST NOT 在 vars 下提供 project、cwd 或其它路径键。由配置模板单测覆盖，MUST NOT 为该插值单独扩 BDD step。

  @req:rc24 @human
  场景: otel-config-section
    - AppConfig MUST 支持可选顶层 otel（或等价）节：至少含 exporter（none 或 otlp-http；缺省等价 none）、可选 endpoint、protocol（http-binary 或 http-json）、environment、service_name 与 headers/凭证引用字段；非法 exporter/protocol MUST 使配置加载失败；该节 MUST 仅控制远程 OTLP 出口，MUST NOT 取代本地 file log / provider-trace 的环境与构建闸（见 infra-logging / infra-provider-trace）。由配置单测覆盖，MUST NOT 为静态字段形状单独扩 BDD step。

  @req:rc25 @human
  场景: tui-editor-history-seed-sessions
    - AppConfig MUST 支持可选顶层 tui 节字段 editor_history_seed_sessions（非负整数）；缺省 MUST 为 1；该值 MUST 供产品 TUI 纯 new session 装载跨 session 发送历史种子（ati41）；由配置单测覆盖，MUST NOT 为静态字段形状单独扩 BDD step。

  @req:rc26 @human
  场景: tool-batch-config
    - AppConfig MUST 支持可选工具批配置节 tool_batch：含 mode（sequential 或 barrier_parallel；缺省 barrier_parallel）；非法 mode MUST 使配置加载失败；显式 sequential MUST 恢复源序串行批；MUST NOT 提供将 mcp_ 工具升为并行安全的配置字段（无 parallel_safe_patterns 或等价 MCP 放行名单）。由配置单测覆盖，MUST NOT 为静态字段形状单独扩 BDD step。

  @req:rc27 @human
  场景: session-max-turns
    - SessionConfig MUST 支持可选 max_turns（正整数）；缺省或省略 MUST 为 None（不限制）；值为 0 或非法 MUST 使配置加载失败；该字段 MUST 供组合根安装 should_stop_after_turn（ar30），MUST NOT 映射为已移除的 max_iterations。由配置单测覆盖，MUST NOT 为静态字段形状单独扩 BDD step。

  @req:rc28 @human
  场景: tui-activity-fold
    - AppConfig MUST 支持可选顶层 tui.activity_fold：enabled（缺省 true）、keep_recent_turns（缺省 2，非负整数）、stream_collapse（envelope 或 clusters，缺省 envelope）、auto_on_rebuild（缺省 true）、auto_on_turn_end（缺省 true）。非法 stream_collapse MUST 使配置加载失败。enabled 为 false MUST 使 ActivityFold 不套信封（全细账，块级折叠仍可用）。由配置单测覆盖，MUST NOT 为静态字段形状单独扩 BDD step。

  @executable @req:rc12
  场景: mode-set
    假如 settings.json 中 steering_mode 设为 one-at-a-time
    当 加载 settings
    那么 Settings.steering_mode 为 OneAtATime

  @executable @req:rc12
  场景: mode-default-one-at-a-time
    假如 settings 未配置 steering_mode 与 follow_up_mode
    当 读取缺省
    那么 二者均为 OneAtATime

  @executable @req:rc13
  场景: settings-delivered-surface-only
    假如 加载默认或示例 settings
    当 检查 Settings 类型与合并结果
    那么 Settings 仅含已接线交付字段

  @executable @req:rc14
  场景: no-prompts-settings-field
    假如 加载默认或示例 settings
    当 检查 Settings 类型与合并结果
    那么 MUST NOT 存在可生效的 prompts 路径列表字段

  @executable @req:rc15
  场景: mapping-documented
    假如 config.yaml 含 compaction 节及 keepRecentTokens
    当 加载配置并解析为运行时 settings
    那么 compaction_settings.keep_recent_tokens 等于 YAML 中设置的值

  @executable @req:rc15
  场景: compaction-model-anchor-alias
    假如 config.yaml 的 compaction.model 经 YAML 别名引用与 models 条目同源的锚点
    当 加载配置
    那么 compaction 任务模型条目与该锚点条目字段一致

  @executable @req:rc15
  场景: compaction-model-settings-ignored
    假如 settings.json 的 compaction 块含 model 或 thinkingLevel 键
    当 加载 settings
    那么 两键被忽略且不进入运行时 CompactionSettings

  @executable @req:rc9
  场景: field
    当 读取默认配置
    那么 mcp 未启用且字段可序列化

  @executable @req:rc16
  场景: parse-list
    假如 YAML 模型条目含 thinking_levels [off, high, max]
    当 加载配置并 resolve_model_meta
    那么 XyModelMeta.thinking_levels 与列表一致

  @executable @req:rc16
  场景: empty-token-fails
    假如 thinking_levels 含空字符串
    当 加载配置
    那么 失败

  @executable @req:rc16
  场景: freeform-ok
    假如 thinking_levels 含厂商字面量 bogon-level
    当 加载配置
    那么 成功且支持集含 bogon-level

  @executable @req:rc16
  场景: default-setting
    假如 Settings.default_thinking_level 为 high 且模型支持集为 off 与 high 与 max
    当 会话首次装配
    那么 当前 thinking level 为 high

  @executable @req:rc16
  场景: select-ignores-settings-default
    假如 Settings.default_thinking_level 为 off 且模型支持集为 off 与 high 与 max
    当 select_model 到该模型
    那么 thinking level 为 max

  @executable @req:rc17
  场景: parse-map
    假如 YAML 含 thinking_levels [off, high] 与 thinking_level_map high: max 与 off: null
    当 加载并 resolve_model_meta
    那么 meta 含 high→max 与 off→null

  @executable @req:rc17
  场景: map-key-outside-list-fails
    假如 thinking_levels 为 [off, high] 且 thinking_level_map 含 max: high
    当 加载配置
    那么 失败

  @executable @req:rc17
  场景: absent-key-ok
    假如 仅配置 thinking_levels 无 map
    当 加载配置
    那么 成功且 map 为空或缺省

  @executable @req:rc18
  场景: tokenizer-hf-ok
    假如 YAML 含 tokenizers.qwen36.repo 且模型条目 tokenizer 为 qwen36
    当 加载配置
    那么 成功且该模型可解析为 HuggingFace 词表源

  @executable @req:rc18
  场景: tokenizer-inline-repo
    假如 模型条目 tokenizer 为 Qwen/Qwen3.6-35B-A3B 字符串
    当 解析 tokenizer 引用
    那么 得到 HuggingFace repo 且无需 tokenizers 表项

  @executable @req:rc18
  场景: tokenizer-unknown-name-fails
    假如 模型条目 tokenizer 为未知名且非 HF repo/路径/builtin
    当 加载配置
    那么 失败

  @executable @req:rc19
  场景: local-tokenizer-default-off
    假如 YAML 未设 token_estimate.local_tokenizer
    当 加载配置
    那么 local_tokenizer 闸为 off

  @executable @req:rc19
  场景: local-tokenizer-on
    假如 YAML 含 token_estimate.local_tokenizer: on
    当 加载配置
    那么 local_tokenizer 闸为 on

  @executable @req:rc19
  场景: local-tokenizer-invalid-fails
    假如 YAML 含 token_estimate.local_tokenizer: every_n
    当 加载配置
    那么 失败

  @executable @req:rc20
  场景: config-yaml-secret-env-layout
    假如 配置加载器已就绪
    当 从全局 config.yaml 加载完整 settings
    那么 全局 config.yaml 生效且不经 config.local.yaml 合并

  @executable @req:rc21
  场景: config-local-not-merged
    假如 仅存在 config.local.yaml 含可观测字段而无同层 config.yaml
    当 加载配置
    那么 该字段不生效（local 被忽略）
