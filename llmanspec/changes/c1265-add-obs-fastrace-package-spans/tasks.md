# Tasks: c1265-add-obs-fastrace-package-spans

status: purpose-draft（Phase A 已收窄）— promote 后再 apply。

## Promote 前

- [ ] 按 Phase A 改写 live `infra-provider-trace`（ipt1/ipt2：跨层低频 span / 关闸零成本；明确不做 map_event/TUI）
- [ ] `llman sdd change attach c1265-add-obs-fastrace-package-spans`
- [ ] validate change + specs

## Phase A 实施

- [ ] 更新 `xylitol-inspect-runtime-logs`：args_delta→mapped 滞后 python 片段 + 闸门提醒
- [ ] agent：`react.stream` / `react.turn`（关闸零成本；`request_id` 属性）；必要时 lifecycle Event 使 JSONL 可见
- [ ] 可选：`tool.execute {name,id}`
- [ ] 单测：闸关无 span/无昂贵路径；闸开一次可见关联字段
- [ ] validate + 相关测试

## 明确不做（本 change）

- [ ] ~~adapter.map_event~~
- [ ] ~~tui.host.event / bridge.apply / engine.frame~~
- [ ] ~~XYLITOL_LIVE_MODEL 真机 CI~~
