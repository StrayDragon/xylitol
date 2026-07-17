# language: zh-CN
# managed by llman sdd partition-migrate
功能: user-experience

  @req:ux0
  场景: placeholder
    假如 无用户交互
    当 无操作
    那么 无变化

  @req:ux1
  场景: login-help
    假如 用户需要 provider 设置引导
    当 get_provider_login_help 被调用
    那么 消息含 /login 与文档路径

  @req:ux2
  场景: no-models
    假如 model registry 为空
    当 format_no_models_available_message 被调用
    那么 '消息含 "/login" 与 "providers.md"'

  @req:ux3
  场景: no-selection
    假如 未选择模型
    当 format_no_model_selected_message 被调用
    那么 '消息含 "Use /login" 与 "/model"'

  @req:ux4
  场景: no-key
    假如 provider anthropic 无 API key
    当 format_no_api_key_found_message 被调用
    那么 '消息含 "anthropic" 与 "/login"'
