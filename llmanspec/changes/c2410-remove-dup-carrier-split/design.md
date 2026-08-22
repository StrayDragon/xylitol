# Design

## 裁决

- **canonical 归属**：载体切分（TUI→四象限 attach / in-process 留给 print·embed·符合性闸）SSOT 定在 `app-tui-bridge atb4`。理由：唯一有可执行 BDD 场景（`remote-type-kept`，断言 `HttpWsClient` 与 `XyInProcessDriver` 双类型存在）；capability 主题即载体/客户端类型。
- **ce6 处置**：删除。其唯一非重叠条款「切换 MUST 只换客户端实现」并入 atb4 statement；其余与 atb4 语义重合。
- **tui-cs 处置**：保留「面本地能力切分」（键盘/绘制/TTY/编辑器/剪贴板 vs host 角色职责），删去句内载体复述；其文档场景 GWT 改为面本地归属表述，不再引用「默认 attach」。
- **不动项**：`server-core sr-remote1`（Host 侧义务）；protocol-app 四象限真源条款。

## 迁移说明

| 删除物 | 替代覆盖 |
|---|---|
| cli-entry `ce6` requirement + driver-abstraction-unit 文档场景 | app-tui-bridge `atb4`（含并入的切换条款）+ `remote-type-kept` BDD |
| tui-cs 句内载体复述 | 同上 |

零产品行为变化；纯合约去重 + 一处测试模块注释更新。
