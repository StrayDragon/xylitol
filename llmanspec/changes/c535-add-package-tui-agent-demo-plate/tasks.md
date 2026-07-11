# Tasks — c535-add-package-tui-agent-demo-plate

- [x] 1. Delta `package-tui-agent-demo`（pad0–pad4）通过 `llman sdd validate c535-add-package-tui-agent-demo-plate --no-interactive`
- [ ] 2. 抽出 `DemoPlate` 项表（id / 标签 / 预制 prompt 或动作枚举）；Ctrl+P 列表由此生成
- [ ] 3. 实现选中路由：md-full / stream-* / diff / tools / tree / help；`/md` `/help` 与 plate 共用
- [ ] 4. 默认 seed 瘦身；footer 去掉键墙（改 plate/help）
- [ ] 5. 更新 `agent_demo_test`；`cargo test -p xylitol-tui --test agent_demo_test`；`llman sdd validate … --strict`
