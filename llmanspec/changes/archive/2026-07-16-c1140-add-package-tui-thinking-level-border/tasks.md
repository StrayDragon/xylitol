# Tasks — c1140-add-package-tui-thinking-level-border

## 1. 库能力

- [x] 1.1 `ThinkingBorderLevel` + `Palette::thinking_border_rgb` / `thinking_border_paint`（off…high 可区分）
- [x] 1.2 Editor：复用 `set_border_color`；可选 `apply_thinking_border` 薄 helper
- [x] 1.3 包单测：parse/cycle；rgb 相邻不等；paint 含真彩 SGR

## 2. agent_demo

- [x] 2.1 demo 持有当前 level；主入口 `Shift+Tab`（plate/`/thinking-level` 可留作辅助）
- [x] 2.2 非 bash 时应用 thinking 边框；bash 优先 success，退出 bash 恢复 thinking
- [x] 2.3 `agent_demo_test`：cycle / Shift+Tab 后边框/level 可断言

## 3. 校验

- [x] 3.1 `LLMANSPEC_BASE_REF=main llman sdd validate c1140-… --strict --no-interactive`
- [x] 3.2 `just test-tui`（或相关包测）+ `just lint`
