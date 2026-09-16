# language: zh-CN
# capability: runtime-model-registry
# purpose: "模型注册表 — provider 注册、鉴权检查、模型发现与解析。Pre-1.0.0 仅支持 OpenAI 兼容与 Anthropic provider。"
# scope: src/agent/model/, src/protocol/

功能: runtime-model-registry

  @req:m1 @human
  场景: ModelRegistry provider
    - ModelRegistry MUST 支持 register_provider(name, config) 与 has_configured_auth(model) / has_resolved_auth(model) 检查 API key 可用性。

  @req:m2 @human
  场景: 模型发现
    - ModelRegistry MUST 支持 get_available()，返回已注册 provider 的全部配置模型，按 provider 优先级排序。

  @req:m3 @human
  场景: 模型默认值
    - ModelRegistry MUST 仅为 openai 与 anthropic provider 维护默认模型 ID。Pre-1.0.0 其它 provider MUST NOT 有默认模型条目。默认模型 ID MUST 为有效、当前可用的模型标识（如 gpt-4o、claude-sonnet-4-20250514），不得为占位或不存在的模型名。该默认 ID 仅作显式解析/回退辅助，MUST NOT 单独构成「用户已配置默认模型」的产品选中态；MUST NOT 因仅存在 provider 环境变量而把该默认 ID 注册进可调用模型表（见 m12/m17）。

  @req:m4 @human
  场景: 模型解析器
    - System MUST 提供 resolve_model(pattern, available)，支持 provider/modelId 规范形式、裸 id 匹配、别名优先于带日期版本、以及 thinking-level:model pattern 解析。

  @req:m5 @human
  场景: 作用域模型
    - System MUST 支持 scoped_models（来自 CLI --models 标志），用于模型循环，将可用模型限制为用户指定子集并可含每模型 thinking level。

  @req:m6 @human
  场景: 模型回退
    - 目标模型 ID 不在 provider 可用列表时，ModelResolver MUST 构建回退 Model，保留用户意图并回退到同 provider 基础模型。

  @req:m7 @human
  场景: 鉴权指引
    - 未配置模型 API key 时，ModelRegistry MUST 返回面向用户的错误消息，指引正确环境变量名。

  @req:m8 @human
  场景: BDD model
    - 模型注册表与解析的 BDD 测试 MUST 全部通过。

  @req:m9 @human
  场景: thinking-levels-on-meta
    - ModelRegistry / resolve_model_meta（或等价）MUST 将配置中的 thinking_levels 字符串列表填入模型 meta 支持集并保留配置顺序；thinking 为 false 或未声明可调列表时支持集 MUST 仅为 off（不可调）。MUST NOT 在未声明列表时静默展开历史 STANDARD（off…high）或其它全球超集；MUST NOT 要求档名属于封闭枚举超集；MUST NOT 依赖 xylitol-tui ThinkingBorderLevel。

  @req:m10 @human
  场景: thinking-level-validate-clamp
    - set_thinking_level（或 Driver 等价）在目标 level 不在当前模型支持集时 MUST 拒绝且 MUST NOT 更改当前值。匹配 MUST 为精确字符串相等（与配置声明字面量一致）；MUST NOT 因仅大小写或首尾空白差异而接受（例如声明 high 时 MUST 拒绝 HIGH 与「 high 」）。换模、精确 /model <id> 或路径要求默认档时：可调模型 MUST 采用支持集配置顺序的末项；仅精确 off 或不可调 MUST 为 off。MUST NOT 因 Settings.default_thinking_level 覆盖换模末项策略。会话 load/resume 还原分支末次 thinkingLevelChange 字符串时 MUST 原样恢复，MUST NOT 因集外而改写 JSONL 或自动追加 clamp 条目；配置漂移时 MAY 短暂 sticky 集外直至用户显式改档。产品面 MUST 展示当前模型声明的档名列表；provider wire 差异经 thinking_level_map 与 api×compat 在请求边界生效。

  @req:m11 @human
  场景: thinking-level-on-generate
    - Agent 在调用 generate_stream（或等价）时 MUST 传入当前 thinking 档名字符串与模型 thinking_level_map（及可用的 thinking_budgets）；resolve 后 MUST 影响下一轮 provider 请求体；MUST NOT 仅更新会话/UI 状态而不影响请求组装。

  @req:m15 @human
  场景: thinking-level-exact-opaque
    - 运行时对 thinking 档名（支持集成员、set/cycle、Settings 默认是否 ∈ 支持集、可调判定中的关档字面量）MUST 使用精确 opaque 字符串；关档约定字面量 MUST 为精确 off。MUST NOT 在匹配或可调判定中对档名做 ASCII 大小写折叠或 trim 归一。

  @req:m12 @human
  场景: no-silent-env-select
    - bootstrap MUST NOT 在缺少用户显式模型别名配置时，仅因存在 provider API key 环境变量而向 ModelRegistry 注册 default_model_id_for_provider（或等价）默认模型；亦 MUST NOT 自动将该默认 ID 设为当前选中模型。无显式模型配置时 registry MUST 为空且经 bootstrap 的表面 MUST 硬失败（见 ce2）；未选中时应用面按 cli-entry ce18 展示。

  @req:m13 @human
  场景: honor-model-entry-api
    - bootstrap / resolve_model_meta（或等价装配）注册 XyModelMeta 时 MUST 保留 ModelsConfig ModelEntry 的可选 api 字段写入 config.api 与 meta.api：省略 api 时 MUST 走 AdapterKind::default_for（或等价）。显式可识别 api（含 openai-completions）MUST 参与 adapter 选择；未识别字符串 MUST 静默等同省略，MUST NOT 为此单独报错。MUST NOT 在注册路径把已配置的 api 字段写成 None。由单测覆盖，MUST NOT 为静态接线单独扩 BDD step。

  @req:m14 @human
  场景: manifest-api-kind-default
    - JSON model manifest 省略 api 时 MUST 按 provider 选用与 AdapterKind::default_for 一致的协议族字符串（OpenAI→openai-responses，Anthropic→anthropic-messages）；显式 openai-completions MUST 可选且参与装配。由单测覆盖，MUST NOT 为静态缺省单独扩 BDD step。

  @req:m17 @human
  场景: explicit-model-entry-auth
    - AppConfig ModelsConfig 中每个显式模型别名条目 MUST 在 bootstrap 注册进 ModelRegistry（供 --list-models 与选模），MUST NOT 因 API key 缺失或未解析而跳过注册。条目 api_key 省略或空串时注册 key MUST 为空；MUST NOT 回落 provider kind 级环境变量（OPENAI_API_KEY / OPENAI_KEY / ANTHROPIC_API_KEY / ANTHROPIC_KEY）。真发请求时缺 key MUST 失败并走鉴权引导（m7/ux4）。由单测覆盖，MUST NOT 为静态边界单独扩 BDD step。

  @req:m18 @human
  场景: 任务级模型解析
    - System MUST 提供按任务用途解析模型的单一入口：给定任务模型条目（provider/model 等连接字段），经注入的 provider 构建器产出独立 XyModel 实例；条目缺失、解析或构建失败时 MUST 回退当前会话模型，且回退 MUST 可观测（归因标注 + 受控频率通知）；任务条目 MUST NOT 注册进 ModelRegistry（不占 --list-models 与模型循环）；MUST NOT 为每个任务角色复制解析逻辑。compact 摘要是第一个消费者（domain-compaction c7）；后续同类旁路任务复用同一入口，不预建未落地任务的抽象。

  @executable @req:m10
  场景: reject-unsupported
    假如 当前模型支持集为 off 与 high
    当 set_thinking_level 为 max
    那么 失败且当前 level 不变

  @executable @req:m15
  场景: reject-case-variant
    假如 当前模型支持集为 off 与 high
    当 set_thinking_level 为 HIGH
    那么 失败且当前 level 不变

  @executable @req:m18
  场景: task-model-resolves-independent
    假如 任务模型条目可构建
    当 经任务级解析入口取模型
    那么 返回独立实例且其模型标识与条目一致

  @executable @req:m18
  场景: task-model-build-failure-falls-back
    假如 任务模型条目构建失败
    当 经任务级解析入口取模型
    那么 回退当前会话模型且回退态可观测
