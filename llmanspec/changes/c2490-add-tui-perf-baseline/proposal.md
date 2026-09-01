---
depends_on: []
skip_specs_landing: true
branch: sdd/c2490-add-tui-perf-baseline
base_sha: 50095ac9a0b48d08e133dc07fd0805362ad18e7c
checkpointed: false
---

# TUI 渲染性能与占用基线：先测量，后优化

## Why

> 定位：**当前最高优先级方向**（人类拍板）。外部成熟引擎的渲染手法调研见
> `research/`，本票先建尺子再谈优化。

产品 TUI 是默认入口，会话会随使用持续增长（长历史、高频工具流式输出），
但当前对两件事**没有量化数据**：

- 流式期间的渲染成本：每帧重绘耗时、事件到达速率与合帧后的实际绘制次数；
- 常驻内存占用：scrollback / 折叠缓存随会话长度增长的曲线。

没有基线就无法回答「卡不卡、占多少」，也无法判断优化是否做过头或做错方向。
仓库已有 lab 手段雏形（`src/app/tui/lab_*.rs` 探针惯例），缺的是系统化口径。

## What Changes

- 建立渲染性能基线测量：以 lab 形态（`lab_*` 惯例，不入 `just qa` 闸）
  输出帧耗时分布（p50/p95/max）、流式场景下每秒绘制次数。
- 建立内存占用画像：固定态会话大小 → 常驻内存的对照表（如 1k/1w/10w 行级 transcript）。
- 测量结果落档到 change research；后续优化票（虚拟挂载、增量重绘、
  缓存上限等候选方向）凭数据立项。

## 非目标

- 本票**不做任何优化实现**——只建尺子；
- 不引入持续基准测试设施（benchmark CI）；
- 不为测量改变生产代码路径的行为。

## Impact

- 扩展既有 `src/app/tui/lab_ao_perf.rs`（帧成本探针已大半存在，见 design 现状核对）；
  新增内存画像探针，不新建 example/binary。
- 无生产行为变化；**无 specs 合约变更 → `skip_specs_landing: true`**（2026-09-01 拍板）。

## 决策记录

- 2026-09-01（ff 深挖拍板）：落点扩展既有 lab 模块；内存口径 `/proc/self/statm` RSS；
  规模档 1k/1w/10w 合成为主；`skip_specs_landing: true`。详见 `design.md`。
- 2026-09-01（apply 发现）：既有两探针原硬编码作者机器的会话 UUID，改为 `XYLITOL_LAB_SESSION`
  显式 opt-in 磁盘会话、缺省合成种子（探针可移植化，属本票 lab 工具范围）。

## Further Notes

- 现有观测底座：`xylitol-inspect-runtime-logs` skill / `just obs-*`；
  本票的测量口径与其互补（面向帧成本而非链路追踪）。
