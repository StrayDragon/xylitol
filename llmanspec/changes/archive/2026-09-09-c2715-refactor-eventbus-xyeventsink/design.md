# Design: EventBus → 只做 XyEventSink

> Designed / pre-start。依赖 `c2700`。

## 1. 目标

进程内事件扇出只有 `XyEventSink`。字符串 `channel` + `Value` 不是产品总线。

## 2. 代码事实

| 事实 | 影响 |
|---|---|
| `EventBus` 已 `impl XyEventSink`（`src/infra/event/mod.rs`） | 可保留为 crate 内默认实现 |
| `settings:changed` 仅出现在 `infra/settings/mod.rs` 注释 | 删注释或改为「未接线，不要实现第二总线」 |
| agent 单测大量 `Arc::new(EventBus::new()) as Arc<dyn XyEventSink>` | 可改 `crate::infra::event::…` 私有类型或 `test_support` |
| `protocol/lifecycle.rs` 提到 EventBus runtime | 改文档为 sink |

## 3. 做法

1. `EventBus` → `pub(crate)`；若名字误导，可改 `FanoutSink`（非必须）。
2. 删除公共 `on(channel)` / 字符串 emit；若测试需要窥探，用 `XyEvent` 录制 fake（已有模式优先）。
3. SettingsManager **不要**为了「注释里的热重载」去接 EventBus。reload 保持文件/显式 `Reload` Command（`c2710` 后）。

## 4. 验证

`rg 'settings:changed'|rg 'EventBus::'` 仅剩实现文件与必要测试构造。`just qa` 相关测。
