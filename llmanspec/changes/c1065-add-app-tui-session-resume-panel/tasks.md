# Tasks — c1065-add-app-tui-session-resume-panel

- [x] 1. Delta + design + future + tasks 齐；`llman sdd validate c1065-add-app-tui-session-resume-panel --no-interactive`
- [x] 2. Driver：`SessionListEntry` 补 `cwd`/`path`；`delete_session`；InProcess + Scripted + Stub/Remote
- [x] 3. 搜索/排序纯函数（`parseSearchQuery` / filterAndSort）+ 单元测试
- [x] 4. P0 面板 UI：header + filter Input + 行渲染；挂到 `EditorSlot::SessionResume`；effects 接线
- [x] 5. P1：rename Input、delete 确认、path 开关；键位表 / DESIGN 更新
- [x] 6. P2：Threaded 折叠态 + fold/unfold 键
- [x] 7. harness：开板 / scope / sort / named / search / rename / delete-reject-current / fold；`PI_DELTAS` 若有差异
- [x] 8. `just fmt` + 相关 test / `just lint`；tasks 全勾后 `llman sdd validate … --strict --no-interactive`
