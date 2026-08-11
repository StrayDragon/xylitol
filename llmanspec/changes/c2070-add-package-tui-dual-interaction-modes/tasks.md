# Tasks: c2070-add-package-tui-dual-interaction-modes

> 验收一致：每条 task 完成 = 对应 seam 绿 + 不越界进 `blocks` 后续 change。
> 垂直切片：调研/规划 → Specs landing → 库 Mode B 子系统 → 产品闸 → 验收。
> **FF / propose**：§1–§2 勾选。**apply**：§3–§5 用条目列表（无 `- [ ]`，避免未实现阶段污染 validate）；进入 `llman-sdd-apply` 后再改勾选清单。

## 1. 升格、调研与规划壳

- [x] 1.1 自 `delayed-changes/tui/` 升格本 change；cascade 五件拆入独立 `llmanspec/changes/<id>/`，frontmatter `depends_on`/`blocks` 正确
- [x] 1.2 补齐 Pi AltScreen / Zellij 选区·滚动·复制·输入一手调研，并写 `research/xylitol-mode-b-subsystem-cut.md`
- [x] 1.3 充实 `proposal.md` / `design.md` / `tasks.md`（本文件）

## 2. Branch binding 与 Specs landing

- [x] 2.1 `llman sdd change attach` 绑定非默认分支 `sdd/c2070-add-package-tui-dual-interaction-modes`
- [x] 2.2 新建 live `package-tui-interaction-modes`；`app-tui-host` 增 `ath30`
- [x] 2.3 合约覆盖：双模式 seam、Mode B 拖选、越界续选、松手复制默认开、输入区排除、产品默认 Mode A、切换换栈；折叠点击不在本 capability
- [x] 2.4 `llman sdd validate … --strict --no-check`；`show --json` 确认 `readyToImplement=true`（specs 已 commit）

## 3. 库：Mode 生命周期与视口（apply）

1. Mode A/B 构造/切换 API：Mode B 进入自管视口（SHOULD alt-buffer）；teardown 恢复；VirtualTerminal 可观测启停序
2. Mode B 复用 `c2020` `enable_mouse_capture`；Moved 仍不强制整帧；禁止第二套 mouse 扇入
3. 应用 ScrollView（或等价）：滚轮/程序滚动；与选区 auto-scroll 共用视口

## 4. 库：选区 / 复制 / 输入排除（apply）

1. 选区状态机：anchor/focus、拖选高亮；松手可配置复制且默认开（OSC52 与/或本地 sink）
2. 越界续选：拖到视口顶/底 → 自动滚 + 扩展选区
3. 输入排除：Editor/Input dock 在 transcript 选区坐标系外（或等价）；指针落输入面不启动 transcript 选区
4. （SHOULD）双击词 / 三击行；为后续折叠 hit 预留「click 消费优先于选区」钩子，不实现折叠

## 5. 产品闸与验收（apply）

1. Host/设置：模式选择；默认 Mode A；切换换栈；Mode B teardown 不残留 mouse/alt
2. 包级单测 + 必要 harness 绿；文档写清 Mode A/B 选区归属 oneof
3. `llman sdd validate … --strict`；确认未实现 `c1760`/`c2040`/`c2050`/`c1505`/`c1535`
