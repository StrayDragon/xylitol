# Tasks: c1500-fix-tui-scrollback-perf

## P0 绘制

- [x] `ScrollbackPaintCache` + `render_scrollback(..., &mut cache)`
- [x] `UiRoot` 持有 cache；theme/fold 失效
- [x] `apply_ui_model` 条件 bump_upper_gen
- [x] 既有 scrollback / ath24 harness 绿灯

## P1 验证

- [x] Harness：多 entry + 多次 streaming 更新 → cache miss / rebuild 有上界
- [x] `tests/tui_e2e` 增加或扩展 `#[ignore]` 大 scrollback 冒烟（pty 或 tmux）
- [x] `just test-tui` 绿；说明 e2e 用 `just test-tui-e2e-pty` / `tmux`

## Specs

- [x] 扩展 `app-tui-host` ath24 或新增 ath25 + feature 场景
- [x] `llman sdd validate c1500-fix-tui-scrollback-perf --strict`
