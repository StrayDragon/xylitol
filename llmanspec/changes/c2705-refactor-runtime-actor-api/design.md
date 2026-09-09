# Design: Runtime actor API

> Designed / pre-start。不 `change start`，直到批次 review 认领。依赖 `c2700` 已落地（`agent` crate-private）。

## 1. 目标

`AgentRuntime` = 会话上的唯一编排对象。`AgentCapabilities` = Runtime 私有聚合，不再是 app 的第二入口。

## 2. 代码事实

| 事实 | 影响 |
|---|---|
| `src/agent/capabilities/mod.rs` 写明 in-scope：model/tools/hooks/session/prompt/compaction/queue；out-of-scope：slash/bang/export（属 Driver） | 本 change **保持**这条切分；只藏类型 |
| `AgentRuntime` 在 `src/agent/runtime/react/mod.rs` | 把今日 `inner.foo` 提升为 Runtime 方法，或 Driver 只持有 Runtime |
| in-process Driver 缓存 Runtime 并直调 Capabilities | 改调 Runtime；禁止 Driver 持有 Capabilities 克隆当稳定句柄 |
| `embed` 暴露 `BootstrappedAgent` | 对外只给 `into_runtime`（若仍需要）/`into_driver`；Capabilities 不可达 |
| `src/AGENTS.md` AgentCapabilities 目标面 | 更新为「crate 内聚合，非库入口」 |

## 3. 公开面（仓内）

Runtime 至少覆盖 Driver 今日经 Capabilities 做的事：

- 模型：current / list / select / cycle / thinking
- 会话：id、fork/switch、messages、stats、tree travel、name、delete（若已在 Capabilities）
- 工具表 freeze / MCP 相关若已在 Runtime 生命周期里，不要再从 Driver 伸进 Capabilities
- compact、queue stats / steer / follow-up
- prompt 装配入口若仅 ReAct 用，不必升到 Runtime pub

Driver 专属仍留 Driver：export HTML/JSONL、bang、slash catalog、clipboard、trust persist、debug scene（debug 删除见 `c2740`）。

## 4. 迁移

1. 列出 `rg 'inner\.' src/app/core/driver` 与 `capabilities.` 外部调用。
2. 缺的 Runtime 方法按调用点补；实现内部仍调 `self.caps`。
3. `pub use AgentCapabilities` 从 `agent/mod.rs` / lib 去掉。
4. 单测：`agent` 层可 `pub(crate)` 构造；`app` 测走 Driver。

## 5. 非目标 / 风险

- 不要为「以后插件」加 trait 对象包装 Capabilities。
- 方法膨胀可暂时发生在 Runtime 上；`c2710` 再用 Command 收口 Driver，Runtime 保持 actor 不是 RPC 表。

## 6. 验证

`just qa`；embed 示例不 `use AgentCapabilities`；`cargo doc` 根页面不出现 Capabilities 为 crate 稳定类型（`pub(crate)` 即可）。
