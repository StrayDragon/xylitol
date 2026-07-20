# language: zh-CN
# managed by llman sdd partition-migrate
功能: runtime-config

  @req:rc0
  场景: placeholder
    假如 未配置任何 settings
    当 无操作发生
    那么 无变化

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
  场景: prompts-list
    假如 prompts 有两个路径
    当 合并 settings
    那么 Settings.prompts 有 2 项

  @req:rc15
  场景: mapping-documented
    假如 用户在 config.yaml 设置 compaction 字段
    当 加载配置并解析为运行时 settings
    那么 规范运行时 compaction settings 经单一文档化映射反映 YAML 值

  @req:rc1
  场景: value-relocated
    假如 检查 src/agent/config_value.rs 是否存在
    当 检查路径
    那么 不存在且存在 src/infra/config/value.rs

  @req:rc8
  场景: config-has-schema
    当 检查 AppConfig 或 settings 类型
    那么 JsonSchema derive 仅出现在配置边界模块

  @req:rc9
  场景: field
    当 读取默认配置
    那么 mcp 未启用且字段可序列化

  @req:rc16
  场景: parse-list
    假如 YAML 模型条目含 thinking_levels [off, high, xhigh]
    当 加载配置并 resolve_model_meta
    那么 XyModelMeta.thinking_levels 与列表一致

  @req:rc16
  场景: unknown-fails
    假如 thinking_levels 含未知名 bogon
    当 加载配置
    那么 失败

  @req:rc16
  场景: default-setting
    假如 Settings.default_thinking_level 为 low 且模型支持 low
    当 启动或选模
    那么 当前 thinking level 为 Low

  @req:rc17
  场景: parse-map
    假如 YAML 含 thinking_level_map high: max 与 off: null
    当 加载并 resolve_model_meta
    那么 meta 含 high→max 与 off→null

  @req:rc17
  场景: unknown-key-fails
    假如 thinking_level_map 含未知名 bogon
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
