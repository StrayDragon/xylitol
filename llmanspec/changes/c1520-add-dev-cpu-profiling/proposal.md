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

- `scripts/profile_tui_suite.py` — 编排（沙箱 HOME/config、种子 jsonl、samply `-p`、录完 SIGINT）
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

## 基线跑次 `suite-20260723-122217`（15s × A–D）

产物：`target/profile/suite-20260723-122217/{A,B,C,D}.{json.gz,summary.txt}`
（commit 后本地目录；不入库。）

### Leaf / bucket（主线程，addr2line）

| 场景 | 主线程 samples | 要点 |
|---|---|---|
| A-idle | 43 | 安静；偶发 `utils` |
| B-scroll | 59 | grapheme ~8%；`strip_ansi` / `grapheme_width` |
| C-stream | 133 | grapheme ~13%；最清晰 |
| D-resume | 379 | 最忙；`utils.rs` 仍在 `$XY` top |

### 调用栈归因（batch addr2line，主线程）

| 场景 | width 相关栈 | scrollback 相关栈 |
|---|---|---|
| C-stream | **~59%** | **~32%**（`scrollback::{fit,markdown}` / wrap） |
| D-resume | **~33%** | ~0%（resume 面板，非 scrollback） |
| B-scroll | ~30% | ~0%（本窗短采样；paint cache 可能已挡重 paint） |
| A-idle | ~26% | ~0%（绝对量低） |

典型链：

- `strip_ansi_codes` / `visible_width` ← **`TUI::do_render`**（硬宽度不变量扫每一行）
- C：另经 `scrollback::fit` / `markdown` / `wrap_text_with_ansi`
- D：另经 `format_session_row_body` / `truncate_to_width`

### 结论（驱动下游 draft）

1. **[c1508](../archive/2026-07-23-c1508-optimize-package-tui-visible-width-ansi/proposal.md)** / **[c1509](../archive/2026-07-23-c1509-optimize-package-tui-wrap-text-ansi/proposal.md)**：已落地并归档。
2. **[c1505](../../do-not-read-me/c1505-add-tui-scrollback-viewport-slice/proposal.md)**（**P9-deferred**）：T0d 后非瓶颈。
3. **A+B finalize 行复用**（`e2da4022`）：`post-ab-e{80,800}` — width ~64%→~37%，主线程样本约减半；体感已可。
4. **[c1535](../../do-not-read-me/c1535-optimize-tui-stream-wrap-tail/proposal.md)**（**P9-deferred**）：下一份额为 wrap/流式尾，ROI 暂低；主线改投 c1495 OTEL/Langfuse 树。
5. Idle 无空转危机；`??` 仍多 → 深挖可开 samply UI 或加 debuginfo。

### 复测 `post-c1508` / `post-c1509`

```bash
python3 scripts/profile_tui_suite.py --build --scenarios C --duration 15 --run-id post-c1508
python3 scripts/profile_tui_suite.py --build --scenarios B,C --duration 15 --run-id post-c1509
samply load target/profile/post-c1509/B-scroll.json.gz
```

- `post-c1508` C：`wrap_text_with_ansi` ~19%（催生 c1509）。
- `post-c1509`：C wrap ~13%；B 短种子 scroll_render ~0%（详见 c1505 T0）。

### Suite 副作用发现

- Resume 帮助行未截断 → 宽度不变量失败 → 卡在 `Loading N/N`（已修，见 `fix(tui): truncate resume help…`）。

## Out of scope

- 改生产默认二进制；CI 硬闸 OS CPU%
- 实现已归档的 c1508/c1509；deferred 的 c1505（见 do-not-read-me）
- 真模型 / MCP 工具链进默认 suite

## 环境前置

```bash
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid
# samply + tmux on PATH
just profile-suite A,B,C,D 15
# or: python3 scripts/profile_tui_suite.py --build --scenarios A,B,C,D --duration 15
```

## 调研备忘

- framehop `two modules at the same start address` = samply stderr，不是产品 bug
- 过滤前 rustc 可占 >90%（agent 工具）；suite 用 Fake 从源头避免
- 热路径：`visible_width` → ANSI 行无 ASCII 快路径 → `strip_ansi` 分配 + grapheme
- profile JSON 内多为 `0x…` 地址；归因需 `addr2line -e target/release/xylitol`

## Status

**purpose-draft** — suite MVP + 首轮基线已记；正式 propose 再收 `profile.profiling` / pprof / criterion。

## Ethics

- risk_level: low
- prohibited_actions: 默认采样拖慢日常路径；把 flamegraph 当 MUST；把工具子进程热点当 TUI 主环
