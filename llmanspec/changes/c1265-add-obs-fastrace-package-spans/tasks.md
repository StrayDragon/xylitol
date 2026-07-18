# Tasks: c1265-add-obs-fastrace-package-spans

status: purpose-draft（Phase A + futures + OTel 钩子文档）— promote 后再 apply。

## Promote 前

- [ ] live `infra-provider-trace`：跨层低频 span / 关闸零成本；注明 OTel 非本 change MUST
- [ ] `llman sdd change attach c1265-add-obs-fastrace-package-spans`
- [ ] validate change + specs

## Phase A 实施

- [ ] 引入 `fastrace-futures`（与 fastrace 版本对齐）；provider Stream `in_span(react.stream)`
- [ ] 更新 `xylitol-inspect-runtime-logs`：滞后 python 片段
- [ ] `react.turn`（+ 可选 `tool.execute`）；关闸零成本；`request_id` 属性
- [ ] 必要时 lifecycle Event 使 JSONL 可见
- [ ] 单测：闸关 / 闸开关联；futures 路径编译与基本行为
- [ ] validate + 相关测试

## 明确不做（本 change）

- [ ] ~~OTel/Jaeger reporter 实现~~（仅 design 钩子）
- [ ] ~~fastrace-reqwest / axum / tracing 兼容层~~
- [ ] ~~adapter.map_event / TUI / engine / live CI~~
