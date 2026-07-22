# Design: c1500-fix-tui-scrollback-perf

## Decisions

### Per-entry paint cache

- Owned by `UiRoot`（与 `upper_cache_*` 并列）。
- Key：`(width, ScrollbackFold, entry_fingerprint)`；fingerprint 覆盖该 `UiEntry` 影响绘制的字段。
- Miss：只重画该 index，并 `truncate` 其后缓存（中间条目变更会失效尾部）。
- Streaming tails：**永不**进 entry cache；仍走现有 Markdown 路径。
- Theme / fold / width 变化：`invalidate()` 整表。

### Upper gen bump

- `apply_ui_model` 比较上区相关字段（`entries`、`streaming_*`、`pending_*`、`queue`）后再 bump。
- `status` / `phase` 单独更新 status_loader，**不** bump upper（ath24）。

### Testing

- **Harness（主闸）**：确定性；用计数器（entry re-render 或 cache miss）证明「N 条历史 + M 次 delta」下 miss ≈ O(M) 而非 O(N·M)。
- **E2E**：`#[ignore]`；造较大 Fake 流或多次事件后仍可 `/exit`；失败判据是超时/挂死，不是 CPU%。

## Alternatives rejected

- 仅调慢 idle tick：不解决 streaming 卡顿。
- 全表 fingerprint 后整表复用：streaming 每字仍整表 Markdown。
- 在 e2e 里采样 `/proc` CPU：环境噪声大，不进 MUST。
