# Tasks: c2070-add-package-tui-dual-interaction-modes

> 验收一致：每条 task 完成 = 对应 seam 绿 + 不越界进 `blocks` 后续 change。
> **范围钉**：本 change 交付 **完整 alt-screen Mode B 库基础**（视口 / transcript 选区 / dock 夹边 / Editor 独立多行选区 / dump / host seam），供产品 TUI 下游迁移。折叠点击仍属 `blocks`。

## 1. 升格、调研与规划壳

- [x] 1.1 自 `delayed-changes/tui/` 升格本 change；cascade 五件拆入独立 `llmanspec/changes/<id>/`，frontmatter `depends_on`/`blocks` 正确
- [x] 1.2 补齐 Pi AltScreen / Zellij 选区·滚动·复制·输入一手调研，并写 `research/xylitol-mode-b-subsystem-cut.md`
- [x] 1.3 充实 `proposal.md` / `design.md` / `tasks.md`（本文件）— 明确「完整 Mode B」交付边界

## 2. Branch binding 与 Specs landing

- [x] 2.1 `llman sdd change attach` 绑定非默认分支 `sdd/c2070-add-package-tui-dual-interaction-modes`
- [x] 2.2 新建 live `package-tui-interaction-modes`；`app-tui-host` 增 `ath30`
- [x] 2.3 合约覆盖至 ptim01–ptim14（含 dock 夹边、Editor 选区、库 host seam）与 ath30
- [x] 2.4 `llman sdd validate … --strict --no-check`；`show --json` 确认 `readyToImplement=true`

## 3. 库：Mode 生命周期与视口

- [x] 3.1 Mode A/B 构造/切换 API：Mode B 进入自管视口（SHOULD alt-buffer）；teardown 恢复；VirtualTerminal 可观测启停序
- [x] 3.2 Mode B 复用 `c2020` `enable_mouse_capture`；Moved 仍不强制整帧；禁止第二套 mouse 扇入
- [x] 3.3 应用 ScrollView + `ModeBRuntime`：绘出行数 ≤ 终端高；滚轮 sticky follow；与选区 auto-scroll 共用视口
- [x] 3.4 suspend/resume（`with_terminal_suspended`）后重新同步 alt + mouse + runtime（ptim11）
- [x] 3.5 Mode B 退出 dump transcript(+dock) 到主屏 scrollback（ptim02）

## 4. 库：transcript 选区 / 复制 / dock

- [x] 4.1 选区状态机：anchor/focus、拖选高亮；松手可配置复制且默认开（OSC52，batch 外）
- [x] 4.2 越界续选：拖到视口顶/底 → 自动滚 + 扩展选区（idle tick）
- [x] 4.3 输入排除：dock 行不进 transcript 选区文本；按下始于 dock 不启 transcript 选区（ptim06）
- [x] 4.4（SHOULD）双击词 / 三击行；折叠 hit 钩子预留，不实现折叠
- [x] 4.5 拖选中进入 dock：夹底边续选，不中途清选；完成选区悬停 dock 不清（ptim12）

## 5. 库：Editor 独立选区（进行中）

- [ ] 5.1 Editor（或共享 Input 缓冲选区类型）支持未修饰拖选，覆盖多行缓冲（ptim13）
- [ ] 5.2 高亮绘在 Editor 可视行；松手复制仅输入文本；与 transcript `SelectionController` 状态隔离
- [ ] 5.3 Mode B：按下始于 dock 时事件回落 Editor；transcript 选区可清；包级单测绿
- [ ] 5.4 `agent_demo` Mode B 路径可人验 Editor 多行选区（`just demo-tui-alt-screen`）

## 6. 产品闸与库 host seam

- [x] 6.1 Host：`TuiRunOptions.interaction_mode`；默认 Mode A；切换换栈；Mode B 登记 dock；不读 `XYLITOL_TUI_MOUSE`
- [x] 6.2 包级单测 + 产品 harness（默认 A / 切 B）绿；`demo-tui-alt-screen` 人验路径
- [ ] 6.3 文档/AGENTS：Mode B 下游接入清单（ptim14）；validate + verify 无 CRITICAL；确认未实现 fold/viewport `blocks`
