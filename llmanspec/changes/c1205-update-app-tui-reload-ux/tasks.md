# Tasks: c1205-update-app-tui-reload-ux

> 规划壳 + Specs landing。**Apply / verify** 见文末清单（无 checkbox，避免未实现前阻塞 `readyToImplement`）。

## 规划壳

- [x] T0: explore 钉案并写入 `proposal.md`
- [x] T1: 写 `design.md`（文案常量、cancel-safe 序、Esc/Ctrl+C、specs 意向）
- [x] T2: 本 `tasks.md`
- [x] T3: review 通过 + `change start` → `sdd/c1205-update-app-tui-reload-ux`

## Specs landing

- [x] T4: live specs — `ath28` / `atc25` / `ati43` / `atm12` 更新
- [x] T5: harness-only（toon `feature: false` unit 场景，对齐 atm12）
- [x] T6: `llman sdd validate … --strict --no-interactive --no-check` 结构绿；commit 后 `readyToImplement=true`

## Apply（`llman-sdd-apply`；非本 landing 勾选闸）

1. Driver/MCP cancel-safe reload（snapshot 换接 + put-back + cancel 响应）
2. host `reload_active` + 软闸 + status `Reloading` + 文案常量
3. effects 非阻塞 `/reload` 泵（ath6 单一入口）
4. harness：进行中 / 拒提交 toast / 取消 / 失败；单测 cancel 收口
5. `status.md`（+ 可选 playground 槽）；`just fmt` / 相关测

## Verify / 收尾

1. `llman-sdd-verify` 证据；手测最短路径
2. `change finalize`（或 checkpoint→archive）
