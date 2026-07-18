# Tasks: c1265-add-obs-fastrace-package-spans

status: full（Phase A 实施中/完成勾选）

## Promote 前

- [x] live `infra-provider-trace`：ipt4 + ipt2/ipt3 收紧
- [x] `llman sdd change attach c1265-add-obs-fastrace-package-spans`
- [x] validate change + specs

## Phase A 实施

- [x] 引入 `fastrace-futures`；provider chunk Stream `in_span(react.stream)`
- [x] 更新 `xylitol-inspect-runtime-logs`：滞后 python 片段
- [x] `react.turn` + `tool.execute`；关闸零成本；`turn_id` 属性（不跨 await 持 LocalParentGuard）
- [x] lifecycle Event 使 JSONL 可见
- [x] 单测：闸关无 span
- [x] validate + 相关测试（提交前）

## 明确不做（本 change）

- [x] ~~OTel/Jaeger reporter 实现~~（仅 design 钩子）
- [x] ~~fastrace-reqwest / axum / tracing 兼容层~~
- [x] ~~adapter.map_event / TUI / engine / live CI~~
