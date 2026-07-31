# Tasks: c1215-update-app-tui-mcp-select-list

## 测试缝（已对齐）

- Host `/mcp`：打开 SelectList；↑↓ 改 selected；Enter / Esc 关槽
- 开槽：在已有 `loaded_resources` 缓存时 **不** 额外 `await loaded_resources_snapshot`（或可观测「同步 mount」）
- 行字段：id / phase / armed / tools；汇总 configured/connected/armed

## 1. Specs + 绑定

- [ ] 1.1 改写 live `atm17`（+ 必要时 ath27）；feature/harness 场景
- [ ] 1.2 `llman sdd change start c1215-…` → `sdd/c1215-…`
- [ ] 1.3 `validate c1215-… --strict --no-check`

## 2. SelectList 壳

- [ ] 2.1 `mcp_list: SelectList`；`mount_mcp_panel` → 从 snap 建 items；render 用 list
- [ ] 2.2 slot_input：↑↓；Enter 关槽；Esc 既有 close
- [ ] 2.3 harness：选中可观测；Enter/Esc 关

## 3. 开槽 perf

- [ ] 3.1 OpenMcp 优先 `UiRoot.loaded_resources`（或 host 缓存）同步 mount
- [ ] 3.2 仅缓存空/无效时 await snapshot；单测或 harness 钉「无二次 await」缝

## 4. 校验

- [ ] 4.1 相关 `cargo test` + `validate --strict`
- [ ] 4.2 playground check 仍绿；短 cue 不回归
