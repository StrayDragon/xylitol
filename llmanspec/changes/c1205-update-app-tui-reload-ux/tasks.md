# Tasks: c1205-update-app-tui-reload-ux

> 规划壳任务。**Specs landing / 实现**须在 Branch binding（`change start` 或 `attach`）之后。

## 规划壳（本波可在默认分支）

- [x] T0: explore 钉案并写入 `proposal.md`
- [x] T1: 写 `design.md`（文案常量、cancel-safe 序、Esc/Ctrl+C、specs 意向）
- [x] T2: 本 `tasks.md`
- [ ] T3: 人工 review 规划壳（你）→ 通过后再 `change start` / `attach`

## Branch binding 后 — Specs landing

- [ ] T4: 在绑定分支改 live specs（按 design「Specs landing 意向」）
  - `app-tui-host`：进行中 UX + 取消/失败可见 + 不堵 tick
  - `app-tui-chrome`：`Reloading` / toast / 无右侧 cue
  - `app-tui-input`：软闸与 Esc/Ctrl+C
  - `app-tui-commands`：atm12 进行中语义（agent busy 拒绝不变）
- [ ] T5: 可执行场景：`.feature` 与/或 harness-only（对齐现 atm12-unit 策略）并 commit
- [ ] T6: `llman sdd validate c1205-update-app-tui-reload-ux --strict --no-interactive` → `readyToImplement=true`

## Apply（ready 后另 skill）

- [ ] T7: Driver/MCP cancel-safe reload（snapshot 换接 + put-back + cancel 响应）
- [ ] T8: host `reload_active` + 软闸 + status `Reloading` + 文案常量
- [ ] T9: effects 非阻塞 `/reload` 泵（ath6 单一入口）
- [ ] T10: harness：进行中 / 拒提交 toast / 取消 / 失败；单测 cancel 收口
- [ ] T11: `status.md`（+ 可选 playground 槽）；`just fmt` / 相关测

## Verify / 收尾

- [ ] T12: `llman-sdd-verify` 证据；手测最短路径
- [ ] T13: `change finalize`（或 checkpoint→archive）
