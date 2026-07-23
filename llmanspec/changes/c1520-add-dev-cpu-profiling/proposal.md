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

## 意向

1. **`[profile.profiling]`**：基于 release、保留 debug 符号、不 strip（正式 promote 时）
2. **可选 feature `cpu-profile`**：`pprof`（非默认）
3. **`just profile-*` + 脚本套件**（**MVP 已落地**）
4. （可选）`criterion` bench

## MVP 已落地（purpose-draft 阶段）

| 命令 | 作用 |
|---|---|
| `just profile-build` | `strip=none` + `debug=line-tables-only` release |
| `just profile-suite` | tmux + Fake + samply attach：A/B/C/D |
| `just profile-summary path` | xylitol-only 摘要 |

脚本：

- `scripts/profile_tui_suite.py` — 编排（沙箱 HOME/config、种子 jsonl、samply `-p`）
- `scripts/summarize_samply_profile.py` — 过滤 agent 拉起的 rustc/cargo/lspz

Fake 环境变量（进程启动前）：

- `XYLITOL_FAKE_SLOW_STREAM=chunks,delay_ms[,chunk_chars]` — C-stream
- `XYLITOL_FAKE_TEXT=...` — 单次短回复

场景：

| ID | 做法 |
|---|---|
| A-idle | attach 空闲 |
| B-scroll | 种子长 session + PPage/NPage |
| C-stream | Fake slow stream + Enter |
| D-resume | 多种子 session + `/session-resume` + Down + Ctrl+U |

**不进 `just qa`**。

## Out of scope

- 改生产默认二进制；CI 硬闸 OS CPU%
- 实现 c1505（需独立证据）
- 真模型 / MCP 工具链进默认 suite

## 环境前置

```bash
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid
# samply + tmux on PATH
just profile-suite
```

## 调研备忘

- framehop `two modules at the same start address` = samply stderr，不是产品 bug
- 过滤前 rustc 可占 >90%（agent 工具）；suite 用 Fake 从源头避免
- 热路径线索：`visible_width` → ANSI 行无 ASCII 快路径 → `strip_ansi` + grapheme

## Status

**purpose-draft** — suite MVP 可用；正式 propose 再收 `profile.profiling` / pprof / criterion。

## Ethics

- risk_level: low
- prohibited_actions: 默认采样拖慢日常路径；把 flamegraph 当 MUST；把工具子进程热点当 TUI 主环
