# language: zh-CN
# capability: test-provider-integration
# purpose: Provider 集成 — API key 解析、模型注册表集成与 OpenAI/Anthropic provider 构造。Attribution headers、OAuth 存储与非 OpenAI/Anthropic provider 集成在 1.0.0 前不在范围内。
# scope: src/infra/provider/, src/protocol/

功能: test-provider-integration

  @req:r1824
  规则: provider-registration
    System SHALL 支持 provider 注册，含可配置 API keys、base URLs、headers 与 adapter api type，适用于 OpenAI 兼容与 Anthropic provider。
    # verified-by: llmanspec/specs/runtime-model-registry/runtime-model-registry.feature

    场景: provider-registration-config
      当 读取 provider 注册配置
      那么 支持密钥与地址与请求头
  @req:r1821
  规则: config-value-parser
    System MUST 提供 ConfigValueResolver，解析 literal、env-var template 与 shell-command 配置值。
    # verified-by: src/infra/config/types.rs

    场景: config-value-parser-boundary
      当 读取配置值解析机制
      那么 支持字面与环境模板解析
  @req:r1822
  规则: env-var-interpolation
    ConfigValueResolver MUST 插值 $VAR、${VAR} 与 ${VAR:-default} 环境变量引用。
    # verified-by: src/infra/config/types.rs

    场景: env-var-interpolation-branch
      当 读取环境变量插值能力
      那么 支持变量引用与默认值
  @req:r1823
  规则: shell-command-execution
    ConfigValueResolver 对以 ! 为前缀的命令 MUST 以 10 秒超时执行并在进程生命周期内缓存结果；超时以失败结果呈现。
    # verified-by: src/infra/config/types.rs

    场景: shell-command-value-exec
      当 读取配置命令执行边界
      那么 命令带超时执行且缓存
  @req:r1825
  规则: provider-in-infra
    所有 LLM provider 实现（OpenAI、Anthropic、Fake、Mock）MUST 位于 infra 层并实现 protocol 端口 XyModel；agent 层 MUST NOT 托管 provider 实现子树；从模型配置到 XyModel 的构造 MUST 位于 infra 层 factory，而非 agent 层。
    # verified-by: src/AGENTS.md

    场景: provider-impl-in-infra
      当 读取 provider 分层
      那么 实现位于 infra 且遵循端口
  @req:r1826
  规则: provider-port-injection
    agent 持有的 model registry MUST 以 Arc<dyn XyModel> 存储 provider；选择 provider MUST NOT 要求 agent 命名具体 provider struct。
    # verified-by: src/AGENTS.md

    场景: provider-port-injection
      当 读取模型注册表存储
      那么 以抽象 trait 对象持有
  @req:r1912
  规则: provider-config-value-expression
    provider 注册配置值（models.models 条目 api_key / model / base_url / api / compat 字段）MUST 支持 ConfigValueResolver 表达式 $VAR、${VAR}、${VAR:-default} 与 !command；纯字面量保持原样；未绑定变量或命令执行失败 MUST 以可读错误拒绝装配（带 alias 与字段上下文）；表达式解析 MUST 与模板渲染（{{ secret.X }} / {{ env.Y }}）共存。
    # verified-by: src/infra/config/loader.rs

    场景: provider-config-value-expression-boundary
      当 读取 provider 注册配置值解析
      那么 展开表达式并兼容字面量
# re-review(c2835): 复审结论——本 capability 管辖行为不变；仅协议载体常量与死变体清理。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）
# re-review(c2853): 承载分支 sdd/2026-10-review-fixes 触及本 scope（BDD 步骤卫生 / 既有 codec·host 改动）；本 capability 管辖行为不变。场景映射不变量保持。（2026-10-09）
# re-review(c2854): 复审结论——本 capability 管辖行为不变；分支改动为 Host 产品入口收口与 BDD 词表，未改本规则可观察语义。（2026-10-09）
