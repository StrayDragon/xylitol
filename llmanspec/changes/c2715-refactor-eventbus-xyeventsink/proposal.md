---
depends_on:
  - c2700-refactor-crate-public-surface
skip_specs_landing: true
---

# EventBus 降为 XyEventSink 实现，去掉第二总线

## Why

`infra::event::EventBus` 是字符串频道 + JSON 的第二套总线，同时又 `impl XyEventSink`。产品事件 SSOT 是 `XyEvent`。`SettingsManager` 模块注释写了热重载走 EventBus `settings:changed`，全仓 **从未 emit** 该频道。发布前应只留 typed sink；字符串总线不再当架构。

## What Changes

- 组合根与 Runtime 只依赖 `Arc<dyn XyEventSink>`（已有 port）。
- `EventBus`：若测试仍需要 in-proc 扇出，改为 **私有** `XyEventSink` 实现，或用已有测试 fake；删除对外 `subscribe(channel: &str)` 产品用法。
- 删除或改写 `settings:changed` 文档；热重载若需要，走现有 config reload / `XyEvent`，不新开字符串频道。
- 不新增 `Xy*` port。

## 非目标

- 不改 `XyEvent` 枚举成员（那是另一条产品 change）。
- 不实现完整 settings 热重载产品（若缺口，另开 change）。

## Capabilities

代码组织 + 死文档。`skip_specs_landing: true`。若 spec 钉 EventBus 频道名，绑定后直接改成 `XyEventSink`。

## Impact

测试里 `EventBus::new() as Arc<dyn XyEventSink>` 可保留实现类型改名。禁止新代码 `bus.emit("settings:changed", …)`。

## 本批依赖

`c2700`：外部不能再 `xylitol::infra::event::EventBus`。可与 `c2705` 并行（不改 Runtime 算法）。
