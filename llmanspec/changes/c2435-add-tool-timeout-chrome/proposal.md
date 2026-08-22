---
depends_on: [c2425-add-timeout-bounds]
---

# 工具行 timeout 预算提示（chrome）

模型显式请求了 `timeout` 的命令类工具（bash/grep/find），其工具行头部在按键提示 `(Alt+E)` 之前以 muted 形式显示 `(timeout {N}s)`（N 为程序钳制后的生效秒数）；走工具默认时不显示。静态文本、无倒计时。

## Why

c2425 落地后 timeout 由程序权威管理，但 TUI 上不可见——运行中用户不知道预算还剩多久、模型为何能跑 10 分钟。头部预算声明让「模型意图 + 程序边界」一眼可见，且只在有信号时出现（默认不显示，避免逐行噪音）。

## What Changes

- `UiEntry::Tool` 新增 `timeout_secs: Option<u64>`；bridge 建条目时对 bash/grep/find 从 args 提取。
- `paint_tool_header_line` 在 hint 前渲染 muted `(timeout {N}s)`；cache hash 同步字段。
- designing：expandable 模块新增 tool-timeout state 稿 + intent 旁注规则补充。
- chrome 词表登记固定表达；跨面衔接注记（gpui 同构）。

## 非目标

fs 四工具（无模型参数）；倒计时；session resume 后的恢复显示（首批 None，恢复路径后续票）。

## Impact

app-tui-chrome 新增 atc27；工具行仅在显式请求时多 ~14 列 muted 文本；零行为变化。
