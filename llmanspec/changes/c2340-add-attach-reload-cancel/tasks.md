# Tasks

测试边界：`HostClient` mock 注入（SnapClient 先例），护栏在 remote 测试模块；不进 BDD feature（ath34-unit 先例）。

## 1. Specs landing

- [ ] 1.1 `change start` 绑定 `sdd/c2340-add-attach-reload-cancel`
- [ ] 1.2 live specs：`app-tui-host` 新增 `ath37`（reload 进行中取消 → 经 Host 合作取消并以 cancelled 收尾）；`server-core` 新增 `sr-abort1`（abort unary 对进程级 reload 的合作取消）
- [ ] 1.3 commit Specs landing；结构过闸 + `readyToImplement`

## 2. 实现

- [ ] 2.1 [blocked-by: 1.3] Remote `reload_runtime`：select cancel vs unary；取消分支 drop 原未来后发 `abort` unary（尽力而为）并返回 `cancelled=true`
- [ ] 2.2 正常完成路径解析逻辑保持不变（现有行为零回归）

## 3. 护栏与收口

- [ ] 3.1 [blocked-by: 2.1] mock HostClient 护栏：reload unary 挂起 → 取消 → 断言 abort 被调用且报告 `cancelled=true`
- [ ] 3.2 fmt/lint/test/bdd 全绿；勾 `_TUI_MIGRATED_TODO.md` P1「C3 reload 合作取消」
