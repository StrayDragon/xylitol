---
depends_on: []
title: TUI 悬停稳定、流式 thinking 可见与 rutis loader 提示降级
needs_specs_change: true
branch: sdd/c3008-fix-tui-hover-thinking-and-rutis-notice
base_branch: main
base_sha: e87f4ab0e49c184bc9ab03d46fb3ffa5f074c699
---

# TUI 悬停稳定、流式 thinking 可见与 rutis loader 提示降级

## Why

- **悬停闪烁**：流式期间每个 `ThinkingDelta` / `TextDelta` 都会让 UI 组件树标记 stale，下一次渲染走整帧重绘；`refresh_pointer_hover_for_paint(false)` 直接清空 hover。鼠标停在可折叠块头时，悬停高亮会随流式帧反复消失/恢复，表现为高频闪烁。
- **流式 thinking 不可见**：ActivityFold 默认折叠 thinking 正文，流式期间也只画簇头；用户要等到思考通道结束（或整轮完成）才能展开看到正文，与“draft reply 时应能看到 thinking”预期不符。
- **rutis loader 提示噪声**：rutis 0.19.1 的 `LoaderPlugin` 提供 `LoaderService`（`#[rutis::service(name = "loader")]`）；本 host 不链接 rutis-bridge，因此 rutis 以 `NotServedByName` 在 ERROR 级报告“service loader is provided by type but not registered by name”。xylitol 不使用按名服务，该提示不可行动，且污染 `xylitol::rutis` 错误日志。

## What Changes

- **悬停稳定**：内容驱动的整帧重绘不再清空 hover；仅在折叠布局变化、resize、overlay 等场景清除。整帧渲染后用最新命中区重新探针，避免悬停目标错位。
- **流式 thinking 可见**：思考通道流式期间，Thinking 正文始终可见（ActivityFold 开关同）；思考通道结束后，落地的 `UiEntry::Thinking` 仍按有效折叠态默认折叠。混合工具簇折叠时，以独立 live Thinking 行显示正文，不自动展开工具簇。
- **rutis 提示降级**：`rutis_error_sink` 对 `ErrorKind::NotServedByName` 降到 debug 级；其余 rutis 异步错误仍为 error。

## Capabilities

- `app-tui-host`（悬停稳定）
- `app-tui-transcript`（流式 thinking 可见）
- `infra-observability`（rutis loader 提示降级）

## Impact

- **行为变化**：流式 thinking 正文默认可见；结束后仍默认折叠。悬停高亮在流式期间保持，折叠点击仍会清除。
- **日志变化**：`NotServedByName` 不再出现在 ERROR 级日志；需要排查时用 `RUST_LOG=xylitol::rutis=debug`。
- **不变**：工具簇默认折叠策略、块级 Alt+E / Ctrl+T 语义、rutis-bridge 不引入。
- **可回退**：恢复整帧重绘清空 hover、恢复流式 thinking 折叠、恢复 `NotServedByName` error 级即可。

## Further Notes

- 已用 tmux + 本地 OpenAI Responses mock 复现：流式 thinking 只显示簇头，正文折叠；`Alt+Shift+E` 展开簇后仍需 `Ctrl+T` 才能看到正文。
- 悬停闪烁由 harness 复现：流式事件触发的整帧重绘后 `pointer_hover_id` 变为 `None`。
- rutis 提示在 `~/.xylitol/logs/xylitol.log` 中以 `ERROR xylitol::rutis` 出现，来源为 `rutis-loader` 的 `LoaderService`。
