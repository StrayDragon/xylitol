# Design：TUI 悬停稳定、流式 thinking 可见与 rutis loader 提示降级

## 1. 悬停稳定

### 现状

- `HostSession::apply_xy_unfenced` 每收到一个 Xy 事件都 `sync_ui_root_from_model()`，后者调用 `TUI::mark_ao_components_stale()`。
- `TUI::do_render` 在 `can_reproject_application_owned()` 为 false 时走整帧重绘，并调用 `refresh_pointer_hover_for_paint(false)`；该函数直接 `pointer.clear_hover()`。
- 流式期间 `ThinkingDelta` / `TextDelta` 频繁触发 stale，鼠标停在折叠块头上时 hover 被反复清空；下一次鼠标移动才恢复，形成闪烁。

### 方案

- `PointerPipeline` 增加 `forget_hover()`：同时清空 `hover_id` 与 `last_cell`，用于布局变化后禁止旧坐标立刻恢复悬停。
- `TUI` 增加 `clear_pointer_hover()`（清 hover 并同步 runtime）与 `reprobe_pointer_hover()`（用当前命中区重探并同步）。
- `refresh_pointer_hover_for_paint`：
  - `can_reproject`：维持现有 `reprobe_hover`。
  - 整帧重绘但宽度未变、无 overlay：改为 `reprobe_hover`，不清空。
  - resize / overlay：`clear_hover()`。
- `HostSession`：
  - 折叠布局变化（`take_fold_dirty()`）时调用 `tui.clear_pointer_hover()`，避免旧 hover 贴到移位后的行。
  - `step_paint_only` 在 `sync_fold_hit_viewport()` 后调用 `tui.reprobe_pointer_hover()`，用最新命中区校正 hover。

### 测试

- harness：鼠标停在 tool 折叠块头 → 注入 `ThinkingDelta` 触发整帧重绘 → `pointer_hover_id` 仍为 Some。
- 现有 `harness_fold_click_clears_stale_hover` 保持：折叠点击后 hover 为 None。

## 2. 流式 thinking 可见

### 现状

- `paint_folded_streaming_thought` 在 ActivityFold 开启时：
  - `live_open_cluster` 且簇未展开 → 直接 return，正文与 L1 头都不画。
  - 簇展开时，正文是否显示取决于 `fold.thinking_effective(STREAMING_THINK_ID)`（默认 false）。
- 结果：思考通道流式期间正文默认折叠；用户需 `Alt+Shift+E` 展开簇后再 `Ctrl+T` 展开 thinking，才能看到正文。

### 方案

- 流式 thinking 是“当前正在发生的活动”，与已结算块不同：流式期间正文始终可见，不依赖 ActivityFold 折叠态。
- `paint_folded_streaming_thought`：
  - `live_open_cluster` 且簇未展开：画独立 live Thinking 行（header + 正文），不展开工具簇。
  - `live_open_cluster` 且簇展开：沿用 L1 Thinking 头，但正文始终画。
  - 非 open cluster：簇头照旧；正文始终画（不再受 `cluster_kids_visible` 限制）。
- 思考通道结束后，`flush_thinking_elapsed` 生成 `UiEntry::Thinking`；`paint_thinking_block` 仍按 `fold.thinking_effective(id)` 默认折叠。
- 保留 `Ctrl+T` 对已结算 Thinking 的 per-id 折叠；流式期间不再需要额外按键即可看到正文。

### 测试

- 更新 `activity_fold_live_thinking_stream_merges_into_thought`：默认即见正文；展开/折叠仍可用。
- 更新 `activity_fold_live_thinking_stream_keeps_own_bar_in_tool_cluster`：折叠工具簇时仍见 live thinking 正文。
- 新增：思考结束后正文按默认折叠态收起。

## 3. rutis loader 提示降级

### 现状

`rutis_error_sink` 对所有 `ErrorKind` 统一 `log::error!`。rutis-loader 的 `LoaderService` 带 `#[rutis::service(name = "loader")]`；host 未链接 rutis-bridge 时，`Ctx::provide` 触发 `ErrorKind::NotServedByName`，被记为 ERROR。

### 方案

```rust
match &*error {
    ErrorKind::NotServedByName { service } => {
        log::debug!(target: "xylitol::rutis", "service {service} ...");
    }
    _ => log::error!(target: "xylitol::rutis", "{error}"),
}
```

- 只降级 `NotServedByName`；其余 rutis 错误保持 error。
- 理由：xylitol 是嵌入式 Rust host，不链接 rutis-bridge，也不使用按名服务；该提示对当前产品不可行动。
- 需要排查时用 `RUST_LOG=xylitol::rutis=debug`。

### 测试

- 单测：sink 收到 `NotServedByName` 时不产生 ERROR 记录，产生 debug 记录；收到 `PluginFailed` 仍产生 ERROR 记录。

## 风险

- 流式 thinking 正文可见会增加瞬时输出量；思考结束后自动折叠，滚动历史仍安静。
- 混合工具簇折叠时插入 live Thinking 行，可能使簇头与 thinking 行之间出现短暂层级跳变；测试覆盖默认与展开两种形态。
- 悬停跨整帧重绘保留，若命中区顺序变化，重探针会在下一帧校正；折叠点击仍显式清除，避免贴错行。

## 非目标

- 不自动展开工具簇。
- 不改变已结算 Thinking 的默认折叠策略。
- 不引入 rutis-bridge 或按名服务。
