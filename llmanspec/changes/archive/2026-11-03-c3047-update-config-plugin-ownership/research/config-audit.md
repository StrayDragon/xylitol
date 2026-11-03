# 配置体系重设计（决策记录）

基线：`1fa9764`。本文件是 c3047 的调研证据，结论已回写 proposal / design。

## 一、已定决策

| # | 决策 | 说明 |
|---|---|---|
| 1 | 全插件化 | 所有配置 section 由其插件声明；core 只保留 loader + 注册表 |
| 2 | 最小引导段 | 保留 `xylitol:` 段，承载 schema 版本、插件启用、未知键策略 |
| 3 | 静态清单落点 | 官方插件 section 清单放 kernel（编译期可见，解决配置加载早于 boot） |
| 4 | 未知键 | 确认默认硬失败；`xylitol.unknown_keys: warn` 为显式逃生口（分析见附录 B） |
| 5 | YAML 复用 | anchor/alias 原生可用；merge key `<<` 由 `Value` 预处理展开；自定义 tag 本期不做 |
| 6 | 机制精简 | 本期移除 `include` / presets / import |
| 7 | schema-first | 删除 `configs/example.yaml` 与生成器；`configs/config.schema.json` 为唯一产物 |
| 8 | CLI 克制 | 只开 `config schema|check|show|init` 四个子命令 |
| 9 | 插件自持配置 | `lsp` / `dap` 等插件自己定义形状；lsp/dap 为语言注册表 |
| 10 | magpie 交付 | 约定目录 `models.d/*.yaml`，不改用户主配置 |
| 11 | 迁移顺序 | 先外放工具类插件，再迁 core 键；键名保持不变 |
| 12 | 外进程插件 section | 允许顶层 section，不强制嵌套；manifest 声明 `config_key` + `config_schema`，插件按 rutis 依赖顺序读取并校验自己的 section |
| 13 | schema_version | 缺失时按当前版本处理 |

## 二、现状盘点

22 个顶层键、135 条叶子路径（附录 A）。顶层键：

`include` / `models` / `agents` / `execution` / `session` / `compaction` / `security` / `tools` / `tool_batch` / `hooks` / `serve` / `otel` / `tui` / `todo` / `workflow` / `plan` / `loop_policies` / `lsp` / `dap` / `subagent` / `external_plugins` / `mcp_servers`。

## 三、逐组问题

### 简单组：include / serve / session / tool_batch / otel / tui

| 键 | 问题 | 建议 |
|---|---|---|
| `include` | 无域片段约定，无来源标注 | **本期移除** |
| `serve` | 只被自动拉起的 Host 读取；显式 `serve` 忽略；非法时长静默回落 30s | 显式 serve 也读，或改名 `host.autostart` |
| `session` | example.yaml 残留死块 `storage` | 保留 `max_turns`，文档写清 0 语义 |
| `tool_batch` | 命名偏实现词，缺使用说明 | 保留，补文档 |
| `otel` | 组合无校验（`otlp-http` 缺 endpoint 运行期才报）；`protocol` 只在 otlp-http 下有意义 | 加组合校验 |
| `tui` | 在 host 侧 AppConfig，但 TUI 是 client | 迁到端侧域 |

### 多实例组：lsp / dap

现状：各持一个 `AgentHandle` 与单份配置（`plugins/xylitol-official-plugin-lsp/src/lib.rs:98-119`、`dap/src/lib.rs:108-125`），只能接一个后端。

佐证：`lspz/src/languages.rs:21-71` 有 `DEFAULT_LANGUAGES`（rust/go/typescript/javascript/python/c/cpp）；`dapz/src/adapters.rs:66-87` 按扩展名返回 `AdapterInfo`。两者本来就是「语言 → 后端」注册表。

建议：

```yaml
lsp:
  servers:
    python: {backend: basedpyright-langserver, args: ["--stdio"], compression: true}
    rust:   {backend: rust-analyzer}
dap:
  adapters:
    python: {backend: "python3 -m debugpy.adapter", cwd: null}
```

未列出的语言沿用内置默认；插件按语言维护 agent 池，工具调用按扩展名路由。

### models / execution / agents（+ settings.json）

| # | 问题 | 证据 |
|---|---|---|
| 1 | 两套配置面：config.yaml（snake_case）+ settings.json（camelCase） | `settings/types.rs:5-21`；`settings/storage.rs:38-39` |
| 2 | settings.json 独有键：default_thinking_level / thinking_budgets / steering_mode / follow_up_mode | `bootstrap.rs:470-500` |
| 3 | settings.json 未知键静默忽略 | `settings/types.rs:81-87` |
| 4 | compaction 两处定义；settings 的 model / thinkingLevel 被忽略 | `compaction_settings.rs:36-59` |
| 5 | `models.models` 双层嵌套 | `types.rs:14-16` |
| 6 | `models.<alias>.fallback` 无消费方 | `model_entry.rs:26` |
| 7 | `context_window: 0` 是魔法值 | `model_entry.rs:105-109` |
| 8 | `api` 自由字符串，未知值静默回落默认 adapter | `provider-openai/src/provider/mod.rs:45-52` |
| 9 | `agents.default_profile` 默认值分歧（serde "default" / Default ""） | `types.rs:376-384` |
| 10 | `agents.default_profile` 指向不存在 profile 时静默回落 | `types.rs:515-524` |
| 11 | `execution` 与 `agents.profiles` 重叠 | `types.rs:352-371,487-506` |
| 12 | 严格性不一致：`AgentProfile` deny，`AppConfig` / `XyModelEntryConfig` 不 deny | `types.rs:361`；`model_entry.rs:12-41` |
| 13 | 本机配置含 `token_estimate` / `tokenizers` 死段 | `~/.config/xylitol/config.yaml` |

解析优先级（`types.rs:487-506`）：`profiles.<name>.model` > `execution.model` > `models.default_model`；CLI `--model` 最高。

## 四、目标形态

```text
AppConfig = { xylitol: BootstrapConfig, sections: BTreeMap<String, Value> }

ConfigSection {
    key:        "lsp" / "models" / "tui"
    owner:      插件 id
    domain:     general | agent | app
    schema:     fn() -> RootSchema
    default:    fn() -> Value
    enabled_by: 有 section 即装配 / 显式 enabled
}
```

- 官方插件：kernel 静态清单。
- 外进程插件：manifest 声明 `config_key` + `config_schema`；允许顶层 section，插件 boot 后按 rutis 依赖顺序读取并校验自己的 section。
- 加载：parse → normalize（merge key）→ `{{ }}` 渲染 → 逐 section 校验 → boot 时插件反序列化自己的 section。
- schema 生成：遍历静态清单；`config schema` 输出聚合或单域片段。

## 五、YAML 能力实测

| 能力 | 结果 | 说明 |
|---|---|---|
| anchor / alias | 支持 | 实测 `b: *base` 正常加载 |
| merge key `<<` | 不支持 | `yaml_serde` 当普通键；需 `Value` 层展开 |
| 自定义 tag `!Tag` | 不支持 | `Value::Tagged` 保留节点，但无内置语义；本期不做 |
| `{{ }}` 模板 | 支持 | minijinja，先渲染再解析 |

## 六、插件配置所有权

| 归属 | 键 |
|---|---|
| core（迁移前） | models / agents / execution / tools / tool_batch / session / security / serve / otel / tui |
| 插件 | compaction / hooks / loop_policies / todo / workflow / plan / subagent / mcp_servers / lsp / dap / external_plugins |

最终全部迁到 owner 插件；core 只留 loader + 注册表 + `xylitol:` 引导段。

## 七、CLI 与未知键

```text
xylitol config schema [--domain] [--ts]
xylitol config check  [path]
xylitol config show   [--resolved]
xylitol config init   [--preset]
```

未知键策略：两阶段。pre-boot 只校验已知官方 section 与 `xylitol:` / `external_plugins` 形状，未知顶层键先收集；boot 后插件按 rutis 依赖顺序校验自己的 section，剩余未知键再硬失败（`warn` 可放宽）。section 内未知键由插件 schema 决定（默认 deny）；未知枚举值硬失败。

## 八、迁移路径

1. P0：schema-first（删 example.yaml）、`config schema|check|show|init`、`xylitol:` 引导段、未知键硬失败。
2. P1：插件自持 section（ConfigSection 注册表 + kernel 静态清单）；先外放工具类插件。
3. P2：core 键迁 owner；`AppConfig` 退化为 section 容器；键名不变。
4. P3：lsp/dap 注册表；magpie `models.d`；settings.json 收口。

## 九、开放问题

1. settings.json：合并进 config.yaml / 只留运行时状态 / 废弃。
2. `models.models`：改 `models.entries` / 扁平 `models: {default, <alias>}` / 保留。
3. `security.enabled`：删除 / 接成总开关。
4. `execution`：合并进 `agents` / 保留。
5. `tui`：迁端侧 / 留 host 侧。
6. magpie `models.d` 约定是否随本期落地。

## 附录 A：全量配置路径

由 `configs/config.schema.json` 递归展开，共 141 条。

  - `agents.default_profile` — string，默认 "default"
      - `agents.profiles.<key>.model` — string，默认 null
      - `agents.profiles.<key>.persona` — string，默认 null
      - `agents.profiles.<key>.system_prompt` — string，默认 null
      - `agents.profiles.<key>.tools[]` — string，默认 []
  - `compaction.enabled` — boolean，默认 null
  - `compaction.keepRecentTokens` — integer，默认 null
    - `compaction.model.api` — string，默认 null
    - `compaction.model.api_key` — string，默认 null
    - `compaction.model.base_url` — string，默认 null
    - `compaction.model.compat` — string，默认 null
    - `compaction.model.context_window` — integer，默认 0
    - `compaction.model.fallback` — string，默认 null
    - `compaction.model.input_modalities[]` — enum(text, image, audio, video)，默认 null
    - `compaction.model.max_output_tokens` — integer，默认 null
    - `compaction.model.model` — string，默认 null
      - `compaction.model.prompt_cache.enabled` — boolean，默认 null
      - `compaction.model.prompt_cache.retention` — enum(in_memory, 24h)，默认 null
      - `compaction.model.prompt_cache.ttl` — enum(5m, 1h)，默认 null
    - `compaction.model.provider` — string，默认 null
    - `compaction.model.thinking` — boolean，默认 true
      - `compaction.model.thinking_level_map.<key>` — string，默认 null
    - `compaction.model.thinking_levels[]` — string，默认 null
  - `compaction.reserveTokens` — integer，默认 null
  - `compaction.thinkingLevel` — string，默认 null
  - `dap.backend` — string，默认 "python3 -m debugpy.adapter"
  - `dap.compression` — boolean，默认 true
  - `dap.cwd` — string，默认 null
  - `execution.model` — string，默认 null
  - `execution.system_prompt` — string，默认 null
  - `external_plugins[].args[]` — string，默认 []
  - `external_plugins[].command` — string，默认 ""
  - `external_plugins[].cwd` — string，默认 null
    - `external_plugins[].env.<key>` — string，默认 null
  - `external_plugins[].name` — string，默认 ""
  - `external_plugins[].timeout_ms` — integer，默认 30000
  - `external_plugins[].trusted` — boolean，默认 false
    - `hooks.global[].command` — string，默认 null
      - `hooks.global[].env.<key>` — string，默认 null
    - `hooks.global[].events[]` — string，默认 []
    - `hooks.global[].phase` — string，默认 ""
    - `hooks.global[].requires_approval` — boolean，默认 false
    - `hooks.global[].timeout_secs` — integer，默认 null
    - `hooks.project[].command` — string，默认 null
      - `hooks.project[].env.<key>` — string，默认 null
    - `hooks.project[].events[]` — string，默认 []
    - `hooks.project[].phase` — string，默认 ""
    - `hooks.project[].requires_approval` — boolean，默认 false
    - `hooks.project[].timeout_secs` — integer，默认 null
    - `hooks.user[].command` — string，默认 null
      - `hooks.user[].env.<key>` — string，默认 null
    - `hooks.user[].events[]` — string，默认 []
    - `hooks.user[].phase` — string，默认 ""
    - `hooks.user[].requires_approval` — boolean，默认 false
    - `hooks.user[].timeout_secs` — integer，默认 null
- `include[]` — string，默认 []
  - `loop_policies.deny_bash_patterns[]` — string，默认 []
  - `loop_policies.enabled` — boolean，默认 false
    - `loop_policies.generator_critic.enabled` — boolean，默认 false
    - `loop_policies.generator_critic.max_rounds` — integer，默认 1
    - `loop_policies.generator_critic.rubric[]` — string，默认 []
  - `loop_policies.max_settles` — integer，默认 null
    - `loop_policies.self_heal.enabled` — boolean，默认 false
    - `loop_policies.self_heal.max_attempts` — integer，默认 3
    - `loop_policies.self_heal.signatures[]` — string，默认 []
    - `loop_policies.tool_exposure.allow[]` — string，默认 []
    - `loop_policies.tool_exposure.deny[]` — string，默认 []
    - `loop_policies.tool_exposure.enabled` — boolean，默认 false
    - `loop_policies.tool_exposure.read_only` — boolean，默认 false
    - `loop_policies.tool_exposure.write_tools[]` — string，默认 []
  - `lsp.backend` — string，默认 "basedpyright-langserver --stdio"
  - `lsp.compression` — boolean，默认 true
  - `lsp.language` — string，默认 "python"
  - `lsp.workspace_root` — string，默认 null
  - `mcp_servers[].args[]` — string，默认 null
  - `mcp_servers[].command` — string，默认 null
    - `mcp_servers[].env.<key>` — string，默认 null
    - `mcp_servers[].headers.<key>` — string，默认 null
  - `mcp_servers[].name` — string，默认 null
  - `mcp_servers[].transport` — enum(stdio, sse)，默认 null
  - `mcp_servers[].url` — string，默认 null
  - `models.default_model` — string，默认 null
      - `models.models.<key>.api` — string，默认 null
      - `models.models.<key>.api_key` — string，默认 null
      - `models.models.<key>.base_url` — string，默认 null
      - `models.models.<key>.compat` — string，默认 null
      - `models.models.<key>.context_window` — integer，默认 0
      - `models.models.<key>.fallback` — string，默认 null
      - `models.models.<key>.input_modalities[]` — enum(text, image, audio, video)，默认 null
      - `models.models.<key>.max_output_tokens` — integer，默认 null
      - `models.models.<key>.model` — string，默认 null
        - `models.models.<key>.prompt_cache.enabled` — boolean，默认 null
        - `models.models.<key>.prompt_cache.retention` — enum(in_memory, 24h)，默认 null
        - `models.models.<key>.prompt_cache.ttl` — enum(5m, 1h)，默认 null
      - `models.models.<key>.provider` — string，默认 null
      - `models.models.<key>.thinking` — boolean，默认 true
        - `models.models.<key>.thinking_level_map.<key>` — string，默认 null
      - `models.models.<key>.thinking_levels[]` — string，默认 null
  - `otel.endpoint` — string，默认 null
  - `otel.environment` — string，默认 null
  - `otel.export_timeout_secs` — integer，默认 60
  - `otel.exporter` — enum(none, otlp-http)，默认 null
    - `otel.headers.<key>` — string，默认 null
  - `otel.observation_io` — enum(none, truncated, full)，默认 null
  - `otel.protocol` — enum(http-binary, http-json)，默认 null
  - `otel.service_name` — string，默认 null
  - `otel.tool_observation_io` — enum(none, truncated, full)，默认 null
  - `plan.allowed_tools[]` — string，默认 []
  - `plan.enabled` — boolean，默认 true
  - `plan.read_only` — boolean，默认 true
  - `security.enabled` — boolean，默认 true
    - `security.permission.backend` — enum(glob)，默认 null
    - `security.permission.enabled` — boolean，默认 false
      - `security.permission.filesystem.read_allowed[]` — string，默认 []
      - `security.permission.filesystem.write_allowed[]` — string，默认 []
      - `security.permission.filesystem.write_denied[]` — string，默认 []
      - `security.permission.network.allowed_domains[]` — string，默认 []
      - `security.permission.network.denied_domains[]` — string，默认 []
      - `security.permission.process.allowed_paths[]` — string，默认 []
  - `serve.idle_timeout` — boolean，默认 null
  - `serve.lifecycle` — enum(last-client, persistent)，默认 null
  - `session.max_turns` — integer，默认 null
  - `subagent.tools[]` — string，默认 ["read", "grep", "find", "ls", "ffgrep", "fffind"]
  - `todo.checkpoint` — boolean，默认 false
  - `todo.enabled` — boolean，默认 true
  - `todo.max_settles` — integer，默认 null
  - `todo.progress` — boolean，默认 false
  - `todo.reminder` — boolean，默认 false
  - `todo.reminder_stale_after` — integer，默认 3
  - `tool_batch.mode` — enum(sequential, barrier_parallel)，默认 null
- `tools[]` — string，默认 []
    - `tui.activity_fold.auto_on_rebuild` — boolean，默认 true
    - `tui.activity_fold.auto_on_turn_end` — boolean，默认 true
    - `tui.activity_fold.enabled` — boolean，默认 true
    - `tui.activity_fold.keep_recent_turns` — integer，默认 2
    - `tui.activity_fold.stream_collapse` — enum(envelope, clusters)，默认 null
  - `tui.editor_history_seed_sessions` — integer，默认 1
  - `workflow.enabled` — boolean，默认 false
  - `workflow.max_depth` — integer，默认 2
  - `workflow.max_replans` — integer，默认 3
  - `workflow.max_total_steps` — integer，默认 100

## 附录 B：未知键策略分析

| 维度 | 硬失败 | warning |
|---|---|---|
| 启动 | 拼错或残留键直接拦住 | 照常启动，问题隐藏 |
| 安全 | `security.permission.enable` 拼错 → 启动失败 | 权限静默关闭 |
| 债务 | 强制清理死段 | 死配置滞留 |
| 升级 | 旧配置需按提示改键 | 部分失效但「能用」 |
| 降级 | 新键在旧二进制上失败 | 新键被忽略 |
| `config check` | 有意义，CI 可卡 | 只能提示 |
| 用户信任 | 报错清晰可接受 | 「配了没生效」难排查 |

硬失败前置条件：注册表覆盖所有合法键；外部插件配置嵌套在 `external_plugins` 条目；未编译/未启用插件给可操作错误；旧键给迁移提示；section 内未知键默认 deny。

## 附录 C：magpie 接入实测

| 检查项 | 结果 |
|---|---|
| `bun src/index.ts status` | 可运行，`models: 0` |
| `MAGPIE_*` 环境变量 | 插件在用（`packages/xylitol/src/shared.ts:6-8`） |
| `magpie-models.yaml` / `magpie.yaml` / `~/.config/x-magpie/config.json` | 都不存在 |
| `config.yaml` 的 `external_plugins` / `include` | 都没有 |
| `secret.env` 的 `MAGPIE_GATEWAY_KEY` | 没有 |

交付建议：约定目录 `~/.config/xylitol/models.d/*.yaml`，加载器并入 `models.models`；插件写这里，不改用户主配置。
