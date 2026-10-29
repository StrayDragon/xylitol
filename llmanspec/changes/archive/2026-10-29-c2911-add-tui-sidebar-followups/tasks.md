# Tasks

- [x] 1. **tui**（pointer）：`HitSpace::Chrome` + `Hit::space`；捕获区 mouse-up 对 chrome 产生 `pending_click`，组件 overlay 行为不变。测：`cargo test -p xylitol-tui pointer`。
- [x] 2. **app-tui**（layout）：侧栏宽度档（24/32/40，内容区 ≥40）、`session_sidebar_width_cycle`、三态循环、侧栏滚动状态与 `overlay_hits` chrome 命中区。测：`cargo test -p xylitol-app-tui`。
- [x] 3. **app-tui**（host）：同步 chrome 命中；点击侧栏/弹出框收起、点击分隔条换宽；滚轮按列分流到侧栏或 transcript。测：`cargo test -p xylitol-app-tui`。
- [x] 4. **specs**（app-tui-fixed-zone）：扩展 `r2903` / `r2904`，写明 chrome 命中、宽度档、三态循环、滚轮边界，并补场景。测：`llman-sdd validate c2911-add-tui-sidebar-followups`。
- [x] 5. **tests**（repo）：补 BDD 场景步骤/绑定与关键单测（宽度循环、三态循环、点击关闭、滚轮不越界）。测：`cargo test --all-features --test bdd`。
- [x] 6. **门禁**（repo）：`llman-sdd validate --strict`、`just qa` 全绿。测：`just qa`。
