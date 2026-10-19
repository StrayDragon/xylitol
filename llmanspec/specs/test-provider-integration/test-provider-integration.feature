# language: zh-CN
# purpose: Provider 集成 — API key 解析、模型注册表集成与 OpenAI/Anthropic provider 构造。Attribution headers、OAuth 存储与非 OpenAI/Anthropic provider 集成在 1.0.0 前不在范围内。

功能: test-provider-integration

  @req:r1824
  规则: provider-registration
    System 须支持 provider 注册，含可配置 API keys、base URLs、headers 与 adapter api type，适用于 OpenAI 兼容与 Anthropic provider。

    场景: provider-registration-config
      当 读取 provider 注册配置
      那么 支持密钥与地址与请求头
  @req:r1821
  规则: config-value-parser
    System 须提供 ConfigValueResolver，解析 literal、env-var template 与 shell-command 配置值。

    场景: config-value-parser-boundary
      当 读取配置值解析机制
      那么 支持字面与环境模板解析
  @req:r1822
  规则: env-var-interpolation
    ConfigValueResolver 须插值 $VAR、${VAR} 与 ${VAR:-default} 环境变量引用。

    场景: env-var-interpolation-branch
      当 读取环境变量插值能力
      那么 支持变量引用与默认值
  @req:r1823
  规则: shell-command-execution
    ConfigValueResolver 对以 ! 为前缀的命令 须以 10 秒超时执行并在进程生命周期内缓存结果；超时以失败结果呈现。

    场景: shell-command-value-exec
      当 读取配置命令执行边界
      那么 命令带超时执行且缓存
  @req:r1825
  规则: provider 走模型端口
    LLM provider 必须实现 XyModel 端口并由装配注入；对话循环不得托管具体厂商实现子树。

  @req:r1826
  规则: provider-port-injection
    agent 持有的 model registry 须以 Arc<dyn XyModel> 存储 provider；选择 provider 不得要求 agent 命名具体 provider struct。

    场景: provider-port-injection
      当 读取模型注册表存储
      那么 以抽象 trait 对象持有
  @req:r1912
  规则: provider-config-value-expression
    provider 注册配置值（models.models 条目 api_key / model / base_url / api / compat 字段）须支持 ConfigValueResolver 表达式 $VAR、${VAR}、${VAR:-default} 与 !command；纯字面量保持原样；未绑定变量或命令执行失败 须以可读错误拒绝装配（带 alias 与字段上下文）；表达式解析 须与模板渲染（{{ secret.X }} / {{ env.Y }}）共存。

    场景: provider-config-value-expression-boundary
      当 读取 provider 注册配置值解析
      那么 展开表达式并兼容字面量
