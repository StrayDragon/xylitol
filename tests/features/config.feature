# language: zh-CN
功能: 配置系统
  作为一个 LLM 代理平台
  我想要 YAML 三层配置
  以便全局、项目和用户级别自定义行为

  场景: 三层配置合并
    假定 全局配置 default_model 为 "gpt-4o"
    并且 项目配置 max_iterations 为 50
    并且 用户配置 default_model 为 "claude-sonnet"
    当 加载配置
    那么 default_model 为 "claude-sonnet"
    并且 max_iterations 为 50

  场景: 模型配置解析
    假定 模型 "gpt-4o" 的 provider 为 "openai"
    并且 模型 "gpt-4o" 的 context_window 为 128000
    当 解析模型配置
    那么 模型 provider 为 "openai"
    并且 模型 context_window 为 128000

  场景: Provider 自定义 base_url
    假定 provider "openai" 的 base_url 为 "https://proxy.example.com/v1"
    当 解析 provider 配置
    那么 base_url 为 "https://proxy.example.com/v1"

  场景: 环境变量插值
    假定 环境变量 OPENAI_API_KEY 为 "sk-test123"
    并且 配置中 api_key 为 "$OPENAI_API_KEY"
    当 解析 api_key
    那么 结果为 "sk-test123"

  场景: 外部命令密钥解析
    假定 配置中 api_key 为 "!echo secret-key-456"
    当 解析 api_key
    那么 结果为 "secret-key-456"

  场景: 设置验证
    假定 配置 compaction_threshold 为 0.8
    当 加载配置
    那么 compaction_threshold 为 0.8

  场景: 缺失配置使用默认值
    当 加载空配置
    那么 max_iterations 为 100
    并且 compaction_threshold 为 0.7
