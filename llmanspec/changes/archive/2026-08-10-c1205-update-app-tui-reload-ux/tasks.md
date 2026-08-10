# Tasks: c1205-update-app-tui-reload-ux

> 规划壳 + Specs landing + Apply。

## 规划壳

- [x] T0: explore 钉案并写入 `proposal.md`
- [x] T1: 写 `design.md`
- [x] T2: 本 `tasks.md`
- [x] T3: review + `change start` → `sdd/c1205-update-app-tui-reload-ux`

## Specs landing

- [x] T4: live specs — `ath28` / `atc25` / `ati43` / `atm12`
- [x] T5: harness-only unit 场景
- [x] T6: validate `--no-check` → `readyToImplement=true`

## Apply

- [x] Driver/MCP cancel-safe reload（snapshot 换接 + put-back + cancel）
- [x] host `reload_active` + 软闸 + status `Reloading` + 文案常量
- [x] effects 非阻塞 `/reload` 泵（`run_interactive_reload`）
- [x] harness：软闸 / 取消 / 失败
- [x] `status.md` Reloading

## Verify / 收尾

1. [x] harness 扩高+中：Reloading 绘制/右 cue、Ctrl+C、二次 slash、bang、Ctrl+G、取消后再 reload
2. [x] verify 缺口修复：overlay Esc、MCP cancel 保 manager、put-back Drop、`McpReloadOutcome`、软闸 `Error:`/无 ScrollNotice、打字、end 清 Reloading
3. [x] `just qa` + commit
4. [ ] `change finalize`（或 checkpoint→archive）
