# Tasks

## 1. SDK 上下文与中间件

- [ ] 1.1 `BeforeLlmRequestCtx` 增加 `Arc<[XyToolDescriptor]>` 运行期 catalog 快照；所有构造点补齐。
- [ ] 1.2 `CodemodeMiddleware` 改为从 ctx 快照生成声明；缓存键覆盖 descriptor 快照（含选择、exposure、freeze 后的结果）；空快照不追加声明。
- [ ] 1.3 保留 inline budget、deferred 回退、稳定排序与不重复工具；`run_code` 自身不进入声明。

## 2. Runtime 与插件接线

- [ ] 2.1 driver 在 freeze / 装配后的 `ToolSet` 上生成一次 catalog，并把 descriptor 快照放进每次 `BeforeLlmRequestCtx`。
- [ ] 2.2 `CodemodePlugin::apply` 删除 `HostRegistries.tools` 声明快照；只注册 `run_code` 与中间件。
- [ ] 2.3 声明、guest `ALL_TOOLS`、dispatch catalog 由同一选择后的工具集派生；reload / freeze / 切会话后快照更新，不粘旧会话。
- [ ] 2.4 无 codemode 配置时零声明、零装配；catalog 为空时声明为空，`run_code` 仍可用。

## 3. 测试

- [ ] 3.1 单测：空 catalog、inline budget、deferred 列表、fingerprint 缓存失效、重复工具去重。
- [ ] 3.2 BDD：session 工具出现在声明；`-tool` 禁用的工具不出现在声明；tool-exposure 过滤同时作用于声明与 catalog。
- [ ] 3.3 回归：无 codemode 配置时无额外声明；其他工具显示与行为不变。

## 4. 门禁

- [ ] 4.1 `just fmt`、相关 clippy、`just test`、`just qa`；BDD 新场景绑定并全绿。
