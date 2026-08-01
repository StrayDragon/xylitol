# Tasks: c1810-add-tui-chrome-footprint

## 测试缝（已对齐 proposal）

- 短终端 busy + Resume / Models：视口内含 `Working`
- toast/queue 占用 reserved 时仍保 status
- 高终端不回归

## 1. Specs landing

- [ ] 1.1 `change start c1810-add-tui-chrome-footprint`
- [ ] 1.2 live：`app-tui-chrome` atc23（footprint + term-aware max_visible）
- [ ] 1.3 validate；commit specs landing

## 2. Manifest + host

- [ ] 2.1 Chrome Footprint 单函数/表（reserved 序对齐词汇表）
- [ ] 2.2 host 注入 `term_rows`；resize 刷新
- [ ] 2.3 `design/chrome-footprint.md`（MUST）+ DESIGN 索引

## 3. 槽接线

- [ ] 3.1 Resume / Tree / Models / MCP / Themes / Import：body 顶 = budget（≥1）
- [ ] 3.2 去掉硬编码 `MAX_VISIBLE*=10` 作为运行时顶（常量可作 default 上限）

## 4. 校验

- [ ] 4.1 解 ignore `busy_resume_short_terminal_keeps_working_in_viewport`；补 Models 短终端用例
- [ ] 4.2 `cargo test` 相关 + `validate --strict`
