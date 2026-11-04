# Tasks — TUI 阶段名展示与首轮触发

- [ ] 1. 模型与事件：`UiModel` / `UiRoot` 增加 `stage_name`；新增 `HostEvent::StageName`；`sync_ui_root_from_model` 同步。
- [ ] 2. 读取：attach / resume 后优先从 entries 的 `SessionInfoEntry.name` 读取，否则经 `get_session_name` unary；不触发自动生成。
- [ ] 3. 首轮触发：首个 `AgentEnd`（fresh、无阶段名、未请求过、非 Aborted/Error）置 pending；effects drain 调 `generate_session_name(force=false)`；每个 session 只一次。
- [ ] 4. Slash：`/stage-name` 查看；`/stage-name <text>` 改名；`/stage-name refresh` 强制刷新；复用 `set_session_name` / `generate_session_name`。
- [ ] 5. 渲染：上边框右端右对齐阶段名；muted；超宽 `…`；无阶段名不留空槽；不增高。
- [ ] 6. Fork / subagent：fork 后读父名；subagent 不显示；失败只显示回退名。
- [ ] 7. Specs：更新 `app-tui-fixed-zone` / `app-tui-commands`，新增裸规则并说明 harness 覆盖。
- [ ] 8. 验证：harness 首轮一次 / resume / slash / 标题截断；snapshot 80/120/160；`llman-sdd validate c3115 --strict`。
