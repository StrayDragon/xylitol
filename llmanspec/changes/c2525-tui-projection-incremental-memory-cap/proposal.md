---
depends_on: []
status: draft
---

# TUI 投影增量化与内存缓存上限（冻结后触发）

> 性质：**草案票（draft）**——只记录问题、数据与触发条件，UX 冻结前不 propose、不实施。

## Why

c2490 基线（归档报告：`llmanspec/changes/archive/2026-09-01-c2490-add-tui-perf-baseline/research/perf-baseline-report.md`，
release 口径）测得两个线性痛点：

- **冷恢复投影 O(行数)**：warm render 1.5k 行 28ms → 15k 行 367ms → 150k 行 **9.96s**；
- **内存线性无上界**：边际 **≈1.42MB/千行组件行**（1w 与 10w 档收敛一致），10w 行 ≈200MB 常驻。

同时数据**否决**了交互层优化需求：wheel/idle 重绘 p50 5–25us、150k 行下 paint_lines 恒等于
终端行数且 40/40 帧 reproject-only——视口投影架构本身达标，不动。

## 触发条件（满足其一才升级为 propose）

1. **UX/信息面冻结里程碑达成**（c2500 协议收敛、c2510 探索分组、c2520 宽度档位表落地，
   transcript segment 语义不再变动）；
2. 真实会话复测越阈：`XYLITOL_LAB_SESSION=<真实uuid>` 跑 `lab_ao_memory_profile`，
   release 下冷恢复 >1s 或常驻 >150MB。

## What Changes（意向，触发后细化）

- 恢复/追加投影增量化：只投影新增与视口邻域，冷恢复分批挂载；
- transcript 缓存上限 + 视口外丢弃（与增量化一并设计，避免二次返工）；
- 尺子随动：c2490 三探针补「增量投影」口径，优化前后对照。

## 非目标

- 不动 steady-state 交互重绘路径（数据已证明达标）；
- 不改落盘结构与回放合约；
- UX 冻结前不动工。
