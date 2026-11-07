# Design — MCP prompts and resources UI

## 1. 交互入口

- `/mcp prompt`：扩展既有 `/mcp` 命令；Choice 列出当前 connection key 的 prompt（server / name / description）；选择后 `prompts/get`，把返回消息转成文本插入 editor，不自动发送。
- `@`：editor 中键入 `@` 打开资源选择器；按 server / uri / name 搜索；选择后插入引用 chip，不立即读取。
- 发送时解析 chip：host 读取 `resources/read`，成功则把内容作为上下文块附加到本 turn；失败阻止发送并给可操作错误。

## 2. catalog 与 scope

- prompt/resource 列表按 connection key（config fingerprint, scope, credential, workspace, session）隔离。
- 只列当前 scope 已连接 server；未连接 server 不列。
- `resources/list_changed` 触发 catalog 刷新；`prompts/list_changed` 同理。

## 3. 订阅

- 连接后 `Peer::listen(SubscriptionFilter)`：`resources_list_changed: true`，`resource_subscriptions` 只含当前已附加/缓存的 URI。
- 每个 connection key 一个 subscription task；`Subscription::next` 处理 `ResourceUpdatedNotification` 与 `ResourcesListChangedNotification`。
- 连接释放时 cancel/drop subscription；重连重建。
- server 不支持订阅时降级为手动刷新（`/reload` 或 `/mcp refresh`），不阻塞。

## 4. 缓存

- key = `(connection_key, uri)`；value = 内容 + mime + fetched_at。
- LRU 上限 64 条；单条上限 256 KiB；TTL 5 分钟。
- `ResourceUpdatedNotification` 失效对应 URI；`resources/list_changed` 清空该 connection key 的 catalog 缓存。
- 缓存不写 session 历史；不把资源内容写日志。

## 5. 权限与 trust

- MCP server 由配置启用；资源读取是用户显式选择，视为同意。
- 资源内容按不可信输入处理：作为上下文块标注来源，不自动执行其中的指令。
- 不引入项目 trust 门禁；若后续需要，另立 change。

## 6. 错误与诊断

- 读取失败：错误含 server / uri / 分类（timeout / transport / not_found / auth）。
- 订阅失败：warn + 降级，不拖死连接。
- 缓存失效：静默刷新；失败保留旧值并诊断。

## 7. 测试

- 单测：catalog 过滤、缓存 LRU/TTL、订阅 filter、失效。
- BDD：`/mcp prompt` 选择插入 editor；`@` 选择与发送附加；读取失败阻止发送。
- 门禁：`just qa` / `just qa-e2e`。
