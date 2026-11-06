# Design — 运行期工具快照驱动的 codemode 声明

## 目标

- 让 codemode 声明、guest `ALL_TOOLS`、dispatch catalog 使用同一份运行期 catalog 快照。
- 消除插件 `apply` 时 `HostRegistries.tools` 快照早于 `compose_tools` 的时序差异。
- 保留 inline budget、deferred 回退、`tools` 选择、tool-exposure / exposure overrides 与确定性排序。

## 现状

- `CodemodePlugin::apply` 在 boot 阶段取 `HostRegistries.tools` 快照生成声明，注册 `CodemodeMiddleware`。
- session 工具由 `compose_tools` 在 driver 装配时注入 `ToolSet`，不进入 `HostRegistries`。
- `tools` 的 `+` / `-` 选择在 `compose_tools` 之后由 `resolve_tool_selection` 执行；插件 `apply` 时的声明快照看不到选择结果。
- `DispatchHandle` 在 `run_code` 执行时用真实 `ToolSet` 建 catalog，guest `ALL_TOOLS` 来自该 catalog。
- 声明文本与可执行 catalog 可能不一致；模型需要额外用 `ALL_TOOLS` / `searchTools` / `describeTool` 发现。

## 为什么需要 catalog / ToolSet

- `tool_schemas` 只是模型可见 schema，已经过 exposure / tool-exposure 过滤；它缺少 deferred 工具、namespace、guidelines、output_schema，无法还原完整声明。
- 声明需要 inline / deferred 分类；guest `ALL_TOOLS` 需要 catalog；dispatch 需要真实 `ToolSet` 才能调用工具。
- 三者若各自从不同来源生成，会再次漂移；必须共享同一份选择后的 catalog 快照。
- 推荐在 `BeforeLlmRequestCtx` 增加 `Arc<[XyToolDescriptor]>` 快照（Clone 便宜、Debug 可行）；`ToolSet` 只留在 dispatch 侧，不必进中间件。

## XyToolSchema 是否重写

- `XyToolSchema` 是 provider 投影（name / description / parameters），保持最小；不要把 catalog metadata 塞进去。
- `XyToolDescriptor` 是 canonical catalog 条目；字段组合保持现状。
- 决定不在 c3218 重写 `XyToolSchema` 或重组 `XyToolDescriptor`；该字段重组是 SDK 公共类型重构，影响所有工具与 provider。
- 如果后续要做，单独开 refactor change，先确认所有 provider / catalog / middleware 消费方。

## 决策

### 1. 生成时机

采用 **BeforeLlmRequest 运行期快照** 为主路径：

- `CodemodeMiddleware` 在每次 LLM 请求前读取 `BeforeLlmRequestCtx` 中的 `Arc<[XyToolDescriptor]>` 快照，生成声明并追加到 `options.system_prompt`。
- driver 在装配完成后把选择后的 catalog 快照放进 `BeforeLlmRequestCtx`；不得回退到插件 `apply` 时的 `HostRegistries` 快照。
- 不在 `apply` 时生成最终声明；插件只注册中间件与 `run_code`。

### 2. 单一 catalog

- 声明、guest preamble、dispatch 都从同一个 `ToolCatalog` 派生。
- 该 catalog 已应用 allowed set、`tools` `+` / `-` 选择、tool-exposure policy、exposure overrides、namespace / identifier 去重。
- 声明与 guest 只消费 `XyToolDescriptor`，不得各自再实现过滤逻辑。

### 3. 缓存与失效

- 缓存键包含 tool table fingerprint + `tools` 选择 fingerprint + freeze generation；同一 turn 内只生成一次。
- freeze、reload、resume、切会话、capability overlay 变化时失效并重算。
- 不得把旧会话声明粘到新会话。

### 4. 顺序与预算

- 先按 namespace / name 稳定排序，再按 inline budget 选择 inline 工具。
- 超出预算的工具进入 deferred 列表；deferred 只列名字与简短描述，不重复 inline 全文。
- 声明不得重复同一工具；`run_code` 自身 `ModelOnly` 不进入声明。
- 被 `-tool` 禁用的工具不得出现在 inline 或 deferred 列表；声明只反映选择后的 catalog。

### 5. 降级

- catalog 不可用或为空时，声明为空；`run_code` 仍可用，guest `ALL_TOOLS` 为空。
- 生成失败只记录诊断，不得阻断 LLM 请求；不得伪造工具列表。

## 数据流

```text
driver assembly
  -> compose_tools -> ToolSet
  -> ToolCatalog::from_toolset
  -> catalog snapshot -> BeforeLlmRequestCtx
  -> codemode middleware (BeforeLlmRequest)
       -> render_declarations(descriptors)
       -> options.system_prompt += declarations
  -> run_code tool -> DispatchHandle -> same catalog -> guest ALL_TOOLS
```

## Seams 与测试

- BDD：启用 todo / workflow 后，声明与 `ALL_TOOLS` 都包含对应工具。
- BDD：tool-exposure read_only 同时过滤声明与 dispatch。
- BDD：`tools: [-bash]` 后声明与 `ALL_TOOLS` 都不含 `bash`。
- 单测：inline budget 边界、deferred 列表、fingerprint 失效、空 catalog。
- 回归：无 codemode 配置时零声明、零装配。

## 风险

- 中间件在请求前生成声明会增加少量 CPU；用 fingerprint 缓存抵消。
- 声明与 guest 若仍各自过滤，可能再次漂移；必须共享 catalog。
- 运行期快照可能包含未冻结工具；声明生成须尊重 freeze 语义。
- 插件激活与工具选择仍分层；本 change 只保证声明基于选择后的 catalog，不解决 `-run_code` 仍启动插件 / `+run_code` 不激活插件的联动。

## 已定决策

- `BeforeLlmRequestCtx` 增加 `Arc<[XyToolDescriptor]>` 快照；不把 `ToolSet` 放进中间件。
- 缓存键包含 `tools` 选择 fingerprint 与 freeze generation。
- catalog 在生成声明前已应用 tool-exposure / exposure overrides，因此声明不依赖 tool-exposure middleware 的 order。

## 迁移

- 不改变 `run_code` schema、guest ABI、权限映射。
- 旧 `HostRegistries` 声明路径删除；不保留双路径。
