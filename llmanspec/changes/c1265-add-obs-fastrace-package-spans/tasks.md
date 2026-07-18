# Tasks: c1265-add-obs-fastrace-package-spans

status: purpose-draft — promote 后再 apply。

## Promote 前

- [ ] live `infra-provider-trace`（及必要）约束：跨层 span MUST / 关闸零成本
- [ ] `llman sdd change attach c1265-add-obs-fastrace-package-spans`

## 实施

- [ ] bridge `adapter.map_event` span
- [ ] agent `react.turn` / `react.stream` / `tool.execute` spans
- [ ] app-tui 采样 span（可配置）
- [ ] 更新 inspect skill 与短脚本：滞后计算
- [ ] opt-in live 剧本（文档 + just 目标）；CI 默认跳过
- [ ] 阶段 B（c1260 后）：真机断言「无裸 tool markup + 意图早于完整 args」
- [ ] validate change
