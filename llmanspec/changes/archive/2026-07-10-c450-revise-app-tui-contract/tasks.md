# Tasks — c450-revise-app-tui-contract

- [x] 1. 更新 `llmanspec/config.yaml` `rules.proposal`：`app-tui-*` / `package-tui-*`；specs 强制中文；其它域前缀意向
- [x] 2. 短更新根 `AGENTS.md` + `src/app/tui/AGENTS.md` + `packages/xylitol-tui/AGENTS.md` 命名指针
- [x] 3. 写入本变更 delta specs（七个 capability）
- [x] 4. 对旧 `app-tui` 执行 remove/modify ops（ratatui / StyledLine / always-on status 等）
- [x] 5. 新建六个 `app-tui-*` **main** spec 壳（`llmanspec/specs/app-tui-*/spec.toon`），供归档合并；delta 已含关键 MUST
- [x] 6. `llman sdd validate c450-revise-app-tui-contract --strict --no-interactive`
- [x] 7. 确认无 `src/app/tui` 产品实现代码混入本变更
- [x] 8. 更新 `_HANDOFF.md` 三轨 DAG
