# Design — app-tui 呈现层状态收敛

## 1. 目标与边界

- 中低风险呈现状态收敛；交互行为与视觉输出不变（needs_specs_change: false）。
- 边界：不动 layout 壳与固定区词表（app-tui-fixed-zone 词汇保留）。

## 2. 决策摘要

| 项 | 方向 |
|---|---|
| 块状态定位 | Bash/Compaction 块按 id 定位（D6）；Compaction 单一写入口（D7） |
| MCP 就绪 | mcp_tools_pending 谓词分顺序重写为显式 ReadyState（D8） |
| 状态栏 | status_busy 不再从文本反推；cue 清除改 id 比较（D9） |
| 折叠 | ScrollbackFold 与 ActivityFold 单一真相（levels 为准，D10/D11） |
| 面板 | 会话恢复面板子状态显式枚举（D12） |
| 拖拽/脏位 | 侧栏拖拽补 Esc/失焦复位（D18）；脏位集合统一置位/消费（D24，查询函数去副作用） |
| 其余 | D13/D20/D21/D22/D23/D25 按扫描点位逐条收敛 |

## 3. 实施顺序

在 c3275/c3280 之后实施避免 seam 冲突；每项独立可回滚。

## 4. 测试边界

harness 快照为主；行为不变由既有 BDD 守住。
