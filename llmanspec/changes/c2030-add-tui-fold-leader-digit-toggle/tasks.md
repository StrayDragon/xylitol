# Tasks: c2030-add-tui-fold-leader-digit-toggle

## 1. Specs landing

- [ ] 1.1 修订 live `app-tui-transcript`：`att7`/`att8` 旁注与 Alt+E 语义对齐 α′；新增 per-block 覆盖 + leader 编号要求（新 req 或扩写）
- [ ] 1.2 修订 live `app-tui-input`：leader / `Alt+Shift+B` / `Alt+Shift+C` 键位合约（新 ati）；`feature: false` 单元场景可
- [ ] 1.3 commit specs → `readyToImplement`

## 2. 数据模型 + 渲染

- [ ] 2.1 `UiRoot`：`tool_block_overrides` + `FoldLeaderMode`；展开判定 helper
- [ ] 2.2 `scrollback` 渲染服从覆盖；fingerprint 含覆盖；单块 toggle 局部 truncate
- [ ] 2.3 Leader 头行编号高亮（仅 mode 中）

## 3. 输入

- [ ] 3.1 keybindings：注册 `foldLeader` / 改 `blocks` 默认 / 新 `compaction.toggle`
- [ ] 3.2 `slot_input` + pre-focus listener：leader / digit / Esc / 首字还焦
- [ ] 3.3 全局 tools 清覆盖；compaction 独立翻转

## 4. 验证与文档

- [ ] 4.1 产品 harness：定点、全局、还焦、compaction 分离、paint miss 上界
- [ ] 4.2 更新 `design/keybindings.md` / `expandable.md` / PI_DELTAS 旁注（若需）
- [ ] 4.3 `just test-tui` 相关 + `llman sdd validate c2030 --strict`
