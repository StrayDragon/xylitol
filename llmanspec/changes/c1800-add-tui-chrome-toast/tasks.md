# Tasks: c1800-add-tui-chrome-toast

## 测试缝（已对齐 proposal）

- harness：busy/bang Resume 拒闸 → toast 有文案 A、无新 ScrollNotice
- toast TTL 可测清除
- travel 等既有尾随 ScrollNotice 不回归

## 1. Specs landing

- [ ] 1.1 `change start c1800-add-tui-chrome-toast`
- [ ] 1.2 live：`app-tui-chrome` 新 req；改写 `atm10` / `ati29`（拒闸 → 壳层通告）
- [ ] 1.3 `validate --strict --no-check`；commit specs landing

## 2. Toast chrome

- [ ] 2.1 host/layout：toast 槽 + `push_chrome_toast` + TTL clear
- [ ] 2.2 渲染在 status/spinner 上方一行
- [ ] 2.3 harness：显示与超时清除

## 3. 迁 c1780 拒闸

- [ ] 3.1 `pending_ui` 改 toast；更新 `c1780_*` harness 断言
- [ ] 3.2 词汇表 / `design` 一句：壳层通告已兑现（现行）

## 4. 校验

- [ ] 4.1 相关 `cargo test` + `validate --strict`
