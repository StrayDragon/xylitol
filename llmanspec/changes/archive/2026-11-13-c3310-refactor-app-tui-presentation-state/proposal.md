---
depends_on: []
title: "Consolidate app-tui presentation state"
needs_specs_change: false
branch: sdd/c3310-refactor-app-tui-presentation-state
base_branch: main
base_sha: 35f49edfee5662acff6f8cded8a70c71d89a0560
---

# app-tui 呈现层状态收敛（P2 草案）

## Why

2026-11-10 扫描的 app-tui 中低风险呈现状态项集中收敛（D6,D7,D8,D9,D10,D11,D12,D13,D18,D20,D21,D22,D24,D25）：

- Bash/Compaction 块状态逆序定位 + Cancelled 覆写过宽（D6）；压缩块两写入口按 tokens_before 去重（D7）。
- MCP 就绪 7 段 if 顺序敏感、分支互相遮蔽（D8）；状态栏 busy 从文本反推、cue 清除靠字符串相等（D9）。
- ScrollbackFold 9 bool + 3 override map 与 ActivityFold levels 两套折叠真相（D10/D11）。
- 面板子状态组合未穷举（D12）；全局粘滞 EXIT_REQUESTED（D13）；侧栏拖拽仅 Up 收尾（D18）；stage_name 三 bool（D20）；boot spinner（D21）；附件双份（D22）；脏位集合含查询内 take（D24）；AskPhase 由结果串反推（D25）。

## What Changes（方向）

- 块状态按 id 定位与显式终态；折叠状态单一真相；面板/拖拽/脏位类型化。

## Impact

- crates/xylitol-app-tui 呈现层。纯呈现收敛，交互行为不变。

## Further Notes

- 与 c3275/c3280 seam 相邻，建议在其后实施避免冲突。
