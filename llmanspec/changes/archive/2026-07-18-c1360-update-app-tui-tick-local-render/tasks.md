# Tasks: c1360

## Specs

- [x] live `app-tui-host`：ath24 Tick 门闩 + 上区缓存
- [x] attach change

## Implement

- [x] HostSession：`paint_dirty`；Tick 条件渲染；`append_bash_chunk` 标 dirty
- [x] UiRoot：upper lines 缓存（width + gen）
- [x] 单测：idle Tick 不强制；busy spinner 仍推进；chunk→Tick 仍绘

## Verify

- [x] `cargo test -q --lib app::tui`
- [x] `llman sdd validate c1360… --strict --no-check`
