# Design — MCP custom transport 插入点

## 1. 目标

- 只保留 transport extension seam；不实现 WebSocket 或任何具体自定义 transport。
- 核心仍只支持 stdio / http；自定义 transport 由插件贡献。
- 不做 API 形状假设：seam 只定义“名字注册 + 连接委托点”，连接器实现留给真实插件。

## 2. 配置

- transport 接受 custom:<name>；name 非空。
- 未注册的 custom name 在配置校验阶段报错（可读、可定位）。
- 已注册但无连接器时，连接阶段返回稳定错误 “custom transport <name> has no connector”；不得静默回退到 http/stdio。

## 3. Registry seam

- McpTransportRegistry：进程级注册表，register(name) / is_registered(name)。
- 插件在装配期注册自己的 transport name；核心只查表与拒绝。
- 不暴露连接器 trait 形状；等第一个真实插件贡献时再定。

## 4. 边界

- legacy HTTP+SSE 仍不支持。
- 不新增 WebSocket 依赖 / 代码。
- 自定义 transport 复用既有 JSON-RPC 语义；不另开第二套消息模型。

## 5. 测试

- 配置：custom:<name> 解析；空 name / 未注册拒绝。
- Registry：注册后校验通过；连接返回 no-connector 错误。
- 核心 stdio/http 行为不变。
