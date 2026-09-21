# AgentStatusBar：核 vs 壳

c2805 落地的是 **核**：`AgentStatusBar` + `project_outbound`，根 `<agent_status_bar>`，Todo 为 `<todo>` 子树，replace-不落盘。

parked `c1895` 多出来的是 **壳**：每跳 persist、独立 session kind、ReadingProvider 注册表、profile × 预算、always-on clock/tool_calls。本波否决。

`/goal` 文本若做：新字段 + `<goal>` 子标签。`/goal` 模式（只规划）：`runtime_policy` + 硬闸，不进栏。
