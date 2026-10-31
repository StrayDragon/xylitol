# 调研：magpie 连接器与插件命令面（2026-10-31）

## magpie 网关

- 默认 `http://127.0.0.1:3425`，Bearer key 默认 `magpie`；env `MAGPIE_GATEWAY_URL` / `MAGPIE_GATEWAY_KEY` / `MAGPIE_GATEWAY_API`。
- 目录：`GET {root}/v1/models`（ETag/304）；行字段 `id`、`display_name`、`context_window`、`max_output_tokens`、`modalities.input`、`reasoning`、`supported_reasoning_levels[{effort}]`、`native_endpoints`、`pricing`。
- 本机实测（`http://tufa:50888`）：7 个模型；OpenAI 族 baseUrl = `{root}/v1`，Anthropic Messages = `{root}`。

## x-magpie 连接器惯例

- 包名 `@x-magpie/<host>`，`type: module`，bun workspace，依赖 `@x-magpie/core`（`fetchCatalog`、`parseRow`、`resolveGateway`、`baseUrlFor`、`applyOverrides`）。
- pi：动态 Provider + `/x-magpie [refresh]`；opencode：内存 transform + `/x-magpie-refresh`；dsh：设置页 plan/apply，只写自己的路由键。
- 密钥来源：opencode `plugins[].options`（本机实测 `~/.config/opencode/opencode.json` 含 `gatewayUrl` 与 `apiKey`）；pi `~/.pi/agent/auth.json` > models.json；dsh 凭据存储。

## xylitol 现状

- 外进程插件：JSON-RPC stdio，manifest 声明 `tools` 能力与工具列表；只能贡献工具，无命令面。
- 斜杠命令：产品 SSOT 静态表（kernel/app-tui 各一份）；`GetCommands` 返回 `CommandInfo`，TUI 目前未消费。
- 配置：三层合并（global → project → CLI），c3021 已加 `include`，可读取插件生成的模型文件。

## 目标流程

1. 连接器插件生成 `~/.config/xylitol/magpie-models.yaml`（不触碰主配置）。
2. 用户主配置 `include: [magpie-models.yaml]`。
3. TUI `/plugin:x-magpie refresh` → 插件拉网关目录、重写文件；`/plugin:x-magpie` → 状态/帮助。
4. 重启或后续模型热重载后生效（v1 以重启为准）。

## 插件命令面设计

- manifest 新增 `capabilities: ["commands"]` 与 `commands: [{name, description, argument_hint?}]`。
- 内核把每个插件注册为 `plugin:<plugin-name>`；参数原样透传给 `commands/call`。
- 协议新增 `Command::PluginCommand { plugin, args }`，方法名 `plugin_command`。
- TUI：解析 `/plugin:*`、目录补全、busy 拒绝、结果以滚动提示呈现（不进模型上下文）。
