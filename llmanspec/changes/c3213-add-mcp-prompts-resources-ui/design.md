# Design — MCP prompts and resources UI

## 1. 用户侧形态

- `/mcp prompt`：打开 prompt 选择器；列出当前 scope 的 prompt（server / name / description）；有参数时用 `ChoicePrompt` 的自定义输入逐项收集，必填为空则提示并重开；`prompts/get` 结果转文本插入 editor，不自动发送。
- `/mcp resource`：打开 resource 选择器；按 server / uri / name 搜索；选中后加入 editor 上方的附件 tray，不插入可解析文本 token。
- `@mcp:`：editor 内输入触发资源补全；选中同样加入附件 tray。未加引号的手写 `@mcp:...` 按普通文本处理，不解析。
- 附件 tray：独立一行，显示 `[server/uri ×]`；支持点击 `×`、Backspace（editor 为空时删最后一个）、快捷键（删最后一个/选中项）；同一 `(server, uri)` 去重。
- 可选显式文本语法 `@mcp:"<server>/<uri>"`：完整引号形式才解析；用于复制/粘贴或纯文本场景；未知资源阻止发送。

## 2. catalog 与 scope

- prompt/resource 列表按 connection key（config fingerprint, scope, credential, workspace, session）隔离。
- 只列当前 scope 已连接 server；未连接 server 不列。
- `resources/list_changed` 触发 catalog 刷新；`prompts/list_changed` 同理。
- TUI 缓存 catalog；host 提供 `mcp.catalog` unary（prompts / resources / counts）。

## 3. context seam

- `XyDriver::run_with_context(prompt, contexts)`；`ResourceContext { server, uri, mime, text, truncated }`。
- Runtime 在用户消息前追加 `SessionEntry::CustomMessage`：`custom_type = "mcpResource"`，`content` 为资源文本，`display` / `details` 为来源元数据。
- `project_for_llm` 把 `CustomMessage.content` 投影为 user text；transcript 渲染可折叠的 context 块（server / uri / mime / size）。
- TUI 发送前逐个调 host `mcp.resource.read`；任一失败阻止发送；成功才调用 `run_with_context`。
- 跨进程经协议 command 载荷携带 contexts；`xylitol-wire` 显式映射。

## 4. 订阅

- 连接后 `Peer::listen(SubscriptionFilter)`：`resources_list_changed: true`，`resource_subscriptions` 只含当前已附加/缓存的 URI。
- 每个 connection key 一个 subscription task；`Subscription::next` 处理 `ResourceUpdatedNotification` 与 `ResourcesListChangedNotification`。
- 连接释放时 cancel/drop subscription；重连重建。
- server 不支持订阅时降级为手动刷新（`/reload` 或 `/mcp refresh`），不阻塞。

## 5. 缓存

- key = `(connection_key, uri)`；value = 内容 + mime + fetched_at。
- LRU 上限 64 条；单条上限 256 KiB；TTL 5 分钟。
- `ResourceUpdatedNotification` 失效对应 URI；`resources/list_changed` 清空该 connection key 的 catalog 缓存。
- 缓存不写 session 历史；不把资源内容写日志。

## 6. 权限与 trust

- MCP server 由配置启用；资源读取是用户显式选择，视为同意。
- 资源内容按不可信输入处理：context 块标注来源，不自动执行其中的指令。
- 不引入项目 trust 门禁；若后续需要，另立 change。

## 7. 引号 token 语法

- 形式：`@mcp:"<server>/<uri>"`；ASCII 双引号；单行。
- server = 第一个 `/` 之前；uri = 之后；server 名不得含 `/`。
- 配置校验：server 名须匹配 `[A-Za-z0-9_-]+`；不得含 `/`、空白、引号、冒号、点号；错误可操作。
- uri 内 `"` 用 `\"` 转义，`\` 用 `\\` 转义；空白可原样出现。
- 解析只在完整闭合引号时生效；跳过 fenced code 与 inline code；`\@mcp:"..."` 表示字面量。
- 未知 server/uri：阻止发送并给可操作错误，不静默当文本。
- 与附件 tray 去重：同一 `(server, uri)` 只读一次。

## 8. 错误与诊断

- 读取失败：错误含 server / uri / 分类（timeout / transport / not_found / auth）；阻止发送。
- 订阅失败：warn + 降级，不拖死连接。
- 缓存失效：静默刷新；失败保留旧值并诊断。
- prompt get 失败：通知条/滚动提示；选择器保留。
- 二进制资源：不可附加，选择器显示 `binary`。

## 9. 测试

- 单测：catalog 过滤、缓存 LRU/TTL、订阅 filter、失效、引号 token 解析（转义/代码块/未知/去重）。
- BDD：`/mcp prompt` 选择 + 参数表单；`@mcp:` 加附件；tray 删除；发送 context；读取失败阻止发送。
- 门禁：`just qa` / `just qa-e2e`。
