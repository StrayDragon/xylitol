# c2923 实施任务

- [ ] 1. 包侧 feature：`package-tui-scrollbar-gutter.feature` 跳顶/跳底/记号/右缘 → 常名（purpose 1、规则 2、步骤短语 3）
- [ ] 2. 包侧 steps：`steps_package_tui_scrollbar_gutter.rs` 3 个 when 短语键同步
- [ ] 3. 主仓 feature：`app-tui-transcript.feature` r2893 规则正文与 product-gutter 场景 when/then 短语同步
- [ ] 4. 主仓 steps：`steps_app_tui_host.rs` 产品 when 短语键同步 + 源码注释 4 处（scroll_gutter.rs/tui/mod.rs/host mod.rs/scroll_view.rs）
- [ ] 5. 词汇表：覆盖弹出框行「右缘浮层」→「右侧浮层」
- [ ] 6. 门禁：`cargo test --test bdd` 全绿 + `llman-sdd validate --all --strict` + 残词 grep 归零
