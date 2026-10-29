# Tasks

## 1. Queue strip 压缩

- [ ] 1.1 `widgets/queue.rs`：steer / follow-up 各最多一行，取最新正文；`len > 1` 时附计数 `(N)`。
- [ ] 1.2 保留 muted 色、Alt+Up 提示与宽度截断。
- [ ] 1.3 单测：多条 steer 只渲染一行且含最新文本与计数；单条不显示计数；空队列返回 0 行。

## 2. 次要文本对比度

- [ ] 2.1 `DESIGN.md` 暗色 `muted` 改为 `#7f849c`；`tool` / `diff-context` 保持不变。
- [ ] 2.2 `Palette::dark()` 同步 `muted`。
- [ ] 2.3 对比度计算与下限断言（≥ 4.3:1），供 BDD 场景使用。

## 3. 规格与 BDD

- [ ] 3.1 `app-tui-input`：更新 `product-queue-fixed-zone`，新增「多条排队压成每类一行」场景。
- [ ] 3.2 `app-tui-fixed-zone`：新增次要文本对比度规则与场景。
- [ ] 3.3 补 step 与 `#[scenario]` 绑定，`cargo test --all-features --test bdd` 全绿。

## 4. 门禁与收尾

- [ ] 4.1 `just qa` 全绿（含 token 闸 / 注释闸 / 版本命名闸）。
- [ ] 4.2 `just qa-e2e` 全绿。
- [ ] 4.3 `llman-sdd validate c2999-update-tui-noise-density --check --strict` 通过。
- [ ] 4.4 verify 报告无 CRITICAL，finalize 归档。
