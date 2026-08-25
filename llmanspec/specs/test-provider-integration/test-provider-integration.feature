# language: zh-CN
# capability: test-provider-integration
# purpose: Provider 集成 — API key 解析、模型注册表集成与 OpenAI/Anthropic provider 构造。Attribution headers、OAuth 存储与非 OpenAI/Anthropic provider 集成在 1.0.0 前不在范围内。
# scope: infra 层 provider, protocol 层

功能: test-provider-integration

  @req:pi0 @human
  场景: provider-registration
    - System SHALL 支持 provider 注册，含可配置 API keys、base URLs、headers 与 adapter api type，适用于 OpenAI 兼容与 Anthropic provider。

  @req:cv1 @human
  场景: config-value-parser
    - System MUST 提供 ConfigValueResolver，解析 literal、env-var template 与 shell-command 配置值。

  @req:cv2 @human
  场景: env-var-interpolation
    - ConfigValueResolver MUST 插值 $VAR、${VAR} 与 ${VAR:-default} 环境变量引用。

  @req:cv3 @human
  场景: shell-command-execution
    - ConfigValueResolver 对以 ! 为前缀的命令 MUST 以 10 秒超时执行并在进程生命周期内缓存结果；超时以失败结果呈现。

  @req:pi2 @human
  场景: provider-in-infra
    - 所有 LLM provider 实现（OpenAI、Anthropic、Fake、Mock）MUST 位于 infra 层并实现 protocol 端口 XyModel；agent 层 MUST NOT 托管 provider 实现子树；从模型配置到 XyModel 的构造 MUST 位于 infra 层 factory，而非 agent 层。

  @req:pi3 @human
  场景: provider-port-injection
    - agent 持有的 model registry MUST 以 Arc<dyn XyModel> 存储 provider；选择 provider MUST NOT 要求 agent 命名具体 provider struct。
