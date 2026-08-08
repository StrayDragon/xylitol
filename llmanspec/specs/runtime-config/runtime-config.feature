# language: zh-CN
# managed by llman sdd partition-migrate
# BDD 接线（tests/bdd.rs）：tokenizer / local-tokenizer 子集已绑
# profile-no-max-iterations / vars-home-unit-covered 在 spec.toon 为 feature:false
# （requirement 明确 MUST NOT 扩 BDD；由配置单测覆盖）
功能: runtime-config

  @req:rc11
  场景: transport
    假如 settings.json 中 transport 设为 sse
    当 加载 settings
    那么 Settings.transport 为 Some(sse)

  @req:rc12
  场景: mode-set
    假如 settings.json 中 steering_mode 设为 one-at-a-time
    当 加载 settings
    那么 Settings.steering_mode 为 OneAtATime

  @req:rc12
  场景: mode-default-one-at-a-time
    假如 settings 未配置 steering_mode 与 follow_up_mode
    当 读取缺省
    那么 二者均为 OneAtATime

  @req:rc13
  场景: shell-path
    假如 settings.json 中 shell_path 设为 "/usr/local/bin/bash"
    当 调用 SettingsManager.get_shell_path
    那么 返回 Some(/usr/local/bin/bash)

  @req:rc13
  场景: trust-default
    假如 default_project_trust 设为 always
    当 调用 SettingsManager.get_default_project_trust
    那么 返回 always

  @req:rc14
  场景: themes-list
    假如 themes 有两个路径
    当 合并 settings
    那么 Settings.themes 有 2 项

  @req:rc14
  场景: no-prompts-settings-field
    假如 加载默认或示例 settings
    当 检查 Settings 类型与合并结果
    那么 MUST NOT 存在可生效的 prompts 路径列表字段

  @req:rc15
  场景: mapping-documented
    假如 config.yaml 含 compaction 节及 keepRecentTokens
    当 加载配置并解析为运行时 settings
    那么 compaction_settings.keep_recent_tokens 等于 YAML 中设置的值

  @req:rc9
  场景: field
    当 读取默认配置
    那么 mcp 未启用且字段可序列化

  @req:rc16
  场景: parse-list
    假如 YAML 模型条目含 thinking_levels [off, high, max]
    当 加载配置并 resolve_model_meta
    那么 XyModelMeta.thinking_levels 与列表一致

  @req:rc16
  场景: empty-token-fails
    假如 thinking_levels 含空字符串
    当 加载配置
    那么 失败

  @req:rc16
  场景: freeform-ok
    假如 thinking_levels 含厂商字面量 bogon-level
    当 加载配置
    那么 成功且支持集含 bogon-level

  @req:rc16
  场景: default-setting
    假如 Settings.default_thinking_level 为 high 且模型支持集为 off 与 high 与 max
    当 会话首次装配
    那么 当前 thinking level 为 high

  @req:rc16
  场景: select-ignores-settings-default
    假如 Settings.default_thinking_level 为 off 且模型支持集为 off 与 high 与 max
    当 select_model 到该模型
    那么 thinking level 为 max

  @req:rc17
  场景: parse-map
    假如 YAML 含 thinking_levels [off, high] 与 thinking_level_map high: max 与 off: null
    当 加载并 resolve_model_meta
    那么 meta 含 high→max 与 off→null

  @req:rc17
  场景: map-key-outside-list-fails
    假如 thinking_levels 为 [off, high] 且 thinking_level_map 含 max: high
    当 加载配置
    那么 失败

  @req:rc17
  场景: absent-key-ok
    假如 仅配置 thinking_levels 无 map
    当 加载配置
    那么 成功且 map 为空或缺省

  @req:rc18
  场景: tokenizer-hf-ok
    假如 YAML 含 tokenizers.qwen36.repo 且模型条目 tokenizer 为 qwen36
    当 加载配置
    那么 成功且该模型可解析为 HuggingFace 词表源

  @req:rc18
  场景: tokenizer-inline-repo
    假如 模型条目 tokenizer 为 Qwen/Qwen3.6-35B-A3B 字符串
    当 解析 tokenizer 引用
    那么 得到 HuggingFace repo 且无需 tokenizers 表项

  @req:rc18
  场景: tokenizer-unknown-name-fails
    假如 模型条目 tokenizer 为未知名且非 HF repo/路径/builtin
    当 加载配置
    那么 失败

  @req:rc19
  场景: local-tokenizer-default-off
    假如 YAML 未设 token_estimate.local_tokenizer
    当 加载配置
    那么 local_tokenizer 闸为 off

  @req:rc19
  场景: local-tokenizer-on
    假如 YAML 含 token_estimate.local_tokenizer: on
    当 加载配置
    那么 local_tokenizer 闸为 on

  @req:rc19
  场景: local-tokenizer-invalid-fails
    假如 YAML 含 token_estimate.local_tokenizer: every_n
    当 加载配置
    那么 失败

  @req:rc20
  场景: config-yaml-secret-env-layout
    假如 配置加载器已就绪
    当 从全局 config.yaml 加载完整 settings
    那么 settings 含 transport 字段且不经 config.local.yaml 合并

  @req:rc21
  场景: config-local-not-merged
    假如 仅存在 config.local.yaml 含可观测字段而无同层 config.yaml
    当 加载配置
    那么 该字段不生效（local 被忽略）
