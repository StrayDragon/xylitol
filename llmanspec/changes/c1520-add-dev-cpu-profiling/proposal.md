---
change_id: c1520-add-dev-cpu-profiling
title: Dev CPU profiling（火焰图 / samply / pprof）
status: purpose-draft
priority: 1520
depends_on: []
author: agent
---

# c1520-add-dev-cpu-profiling

## Why

TUI 长会话性能优化（c1500 / c1510 / 后续 c1505）目前仍大量依赖**体感**与 harness 计数。需要可复现的 CPU 热点图，避免「感觉卡」与真实热点错位。

## 意向（延后正式 propose）

1. **`[profile.profiling]`**：基于 release、保留 debug 符号、不 strip
2. **可选 feature `cpu-profile`**：`pprof` 采样，退出或信号写出 `target/profile/*.svg` / pb
3. **`just profile-*`**：文档化 `samply record` / `cargo flamegraph`（零强制 deps）与可选 pprof 路径
4. （可选）`criterion` bench：`render_scrollback` / streaming assistant paint
5. **摘要脚本**（已有）：`scripts/summarize_samply_profile.py` — 默认过滤 agent 拉起的 cargo/rustc/lspz

## Out of scope（本 draft）

- 改生产默认二进制行为
- 以 OS CPU% 作 CI 硬闸
- 实现 c1505 viewport slice（需独立证据）

## 提案顺序（人类约定）

1. **本 change（c1520）** — 调研 / 文档 / 脚本 → 再 promote
2. **c1505** viewport slice（若 profile 证明长历史 flatten 是瓶颈）
3. 其他

## 调研备忘（2026-07-23）

### 环境

- `samply 0.13.1`；Linux 需 `perf_event_paranoid ≤ 1`
- release 默认 `strip = "symbols"` → 采盘前用
  `CARGO_PROFILE_RELEASE_STRIP=none CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`
- framehop 警告 `two modules at the same start address` = **samply stderr**，不是产品 bug；重定向 `2> target/profile/samply.err`

### 进程过滤（重要）

一次会话里若模型调用了工具（lspz / cargo check / RA），samply 会采到 **rustc/cc1/cargo**，可占全 profile **>90%** samples。
分析 **TUI 主环**时必须：

```bash
python3 scripts/summarize_samply_profile.py target/profile/FOO.json.gz \
  --addr2line ./target/release/xylitol \
  -o target/profile/FOO.summary.txt
```

（默认只保留 `processName=xylitol`。）

### 初步热点（过滤后）

- 叶子大量 `unicode-segmentation` / grapheme / str
- 产品侧可见 `packages/xylitol-tui/src/utils.rs`：`strip_ansi_codes` ← `visible_width`
- 机制：`visible_width` 的 ASCII 快路径在**含 ANSI 的已上色行**上失效 → 每行测宽走 strip + grapheme
- 调用面广：引擎差分渲染（`tui.rs`）、Markdown、scrollback pad、Resume 等

### 安静 profile 提示（幂等）

见下方「操作流程」；避免触发会 spawn 编译器的工具。

## Status

**purpose-draft** — 先采干净场景对照，再决定 promote 范围（`just profile-*` vs in-process pprof vs 测宽优化 change）。

## Ethics

- risk_level: low
- prohibited_actions: 默认开启采样拖慢日常路径；把 flamegraph 当作 MUST 产品行为；把 agent 工具子进程热点误判为 TUI 主环
