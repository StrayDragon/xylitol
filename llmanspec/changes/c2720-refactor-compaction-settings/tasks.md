# Tasks: c2720-refactor-compaction-settings

> pre-start。依赖 `c2700`。

## 1. 测绘

- [ ] 1.1 列出 `CompactionSettingsSchema` / `XyCompactionSettingsConfig` / `CompactionSettings` 全部 From 与 schema derive。
- [ ] 1.2 [blocked-by: 1.1] 确认 SettingsManager 是否仍读写 compaction；决定删闲置 vs 保留 merge。

## 2. 折叠

- [ ] 2.1 [blocked-by: 1.2] schema 改为 derived；删除手写孪生字段。
- [ ] 2.2 [blocked-by: 2.1] 单一 `From`；bootstrap 只走该路径。
- [ ] 2.3 [blocked-by: 2.2] 真死的 SettingsManager API 按 dead-code skill 删。

## 3. 验证

- [ ] 3.1 [blocked-by: 2.3] config validate + compaction settings 单测。
- [ ] 3.2 [blocked-by: 3.1] `just qa` 触及面；`llman sdd validate c2720-refactor-compaction-settings --strict --no-check`。
