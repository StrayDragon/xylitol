# tmux 观测 — 流式 CPU

日期：2026-11-04；环境：macOS，tmux 3.8；debug / release 构建；本地 Responses SSE 假模型。

## 装置

- `/tmp/xyl_lab5.py`：1200 thinking delta + 300 text delta，间隔 1ms（约 1000 delta/s）；`xylitol serve` + `xylitol tui` 在 tmux；采样 `ps -o cputime`（TUI 与 tmux server）。
- `/tmp/xyl_lab2.py`：40ms 间隔（25 delta/s）；`/tmp/xyl_lab3.py`：4ms（250 delta/s）+ 80 轮长会话。
- 渲染 lab：`cargo test -p xylitol-app-tui lab_ao_stream_delta_perf_report -- --ignored --nocapture`。

## 数字

| 场景 | debug | release |
|---|---|---|
| 25–250 delta/s | ~1% | 未测 |
| 1000 delta/s | 5.53s CPU / 5.54s（~100%） | 1.66s / 1.97s（~85%） |
| tmux server | ~1% | ~1–3% |

渲染 lab：

- 160 delta 逐 token：160 帧，Σ do_render 186,092µs，851 帧/s。
- 8 delta 合帧：20 帧，Σ 22,833µs；8.15× 下降。

## 机制

- runtime 每 chunk 先 `TextDelta` 再 `MessageUpdate`（`react/mod.rs:1375-1381`）。
- TUI 合批只认 `TextDelta`/`ThinkingDelta`（`lib.rs:644`）；`MessageUpdate` 打断。
- 每 delta 全量 `sync_ui_root_from_model`（`host/mod.rs:845`）+ clone `UiModel` + bump upper cache + 重建 scrollback + 指纹。
- `mark_ao_components_stale` 绕过 16ms 节流；busy tick 16ms。

## 结论

- 高 token 速率下 CPU 打满单核，release 仍 ~85%；不是 debug-only。
- 合批 `MessageUpdate` + 40ms 窗口是首要修复；tail cache / tick / fingerprint 次之。
- 目标：release 1000 delta/s <20%；复跑 `/tmp/xyl_lab5.py` 验收。

## 注意

- macOS 本机；Windows / 其他终端未验证。
- `ps -o cputime` 分辨率 0.01s；采样 0.2s。
- 假模型速率可控，真实 provider 速率不同；结论是机制与量级，不是绝对百分比承诺。
