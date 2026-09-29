---
depends_on: []
---

## Why

批 3 遗留：r1415（压后地板一次性诊断）的触发判定内联在 orchestrator 压后收尾，无法在 BDD 里构造「压后仍满窗」。

## What Changes（设想）

- 提取压后地板诊断判定为可注入缝（或 BDD 用「配置 contextWindow 极小的模型 + 50 轮会话」注入满窗场景）。
- 断言 CompactionEnd notice 恰发一次地板诊断（含建议文案），manual 路径不发，每 run 至多一次。

## Evidence

c2829 research/triage.md；orchestrator.rs floor_diagnostic_notice。
