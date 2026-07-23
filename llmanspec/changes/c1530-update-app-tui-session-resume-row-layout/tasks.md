# Tasks: c1530-update-app-tui-session-resume-row-layout

## Specs / design

- [x] 更新 `app-tui-commands` atm10：预览比例软顶；默认不展示 session id；显示时完整不截断
- [x] 更新 `app-tui-input` ati29：Ctrl+U / `app.session.toggleId` toggle id 列
- [x] feature 场景：默认隐藏 id；Ctrl+U 展开完整 id
- [x] `design/session-resume.md` + `keybindings.md` + `DESIGN.md` 索引对齐
- [x] `llman sdd change attach` + `validate --strict`

## Implement

- [ ] 注册 `app.session.toggleId` → `ctrl+u`；面板 `show_id` 状态（默认 false）
- [ ] 行布局：预览软顶 ≈60% 终端宽；`show_id` 时中间列完整 id；header 提示 `(ctrl+u)`
- [ ] 单测：默认行不含完整 uuid；toggle 后含；窄宽不截断 id
- [ ] harness：Ctrl+U 显隐（可挂 h37 或独立）

## Playground

- [ ] playground 增加 Resume 槽：默认态 / id-on 芯片切换
- [ ] `design/fixtures/session-resume.*.yaml` + `check_tui_design_playground` 绿

## Verify

- [ ] `cargo test --lib session_resume` + 相关 harness
- [ ] `python3 scripts/check_tui_design_playground.py --check`
- [ ] `llman sdd validate c1530-update-app-tui-session-resume-row-layout --strict --no-interactive`
