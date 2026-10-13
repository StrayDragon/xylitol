# Tasks

- [ ] T1 specs：`app-tui-storage` r2902 补「并入后召回顺序 须 最新优先」条款 +
      场景 `editor-history-merge-recall-order`（GWT 复用既有存储通道步骤，
      新增「先旧后新写入 + 召回文本」两步）。
- [ ] T2 修复：`merge_client_history` 反转通道序（新→旧 → 旧→新）传入
      `merge_editor_history`；包层 `merge_history` doc 契约改 oldest→newest
      （循环体不动）。[blocked-by: T1]
- [ ] T3 验证：包层顺序单测（现史 + 外部块序、去重、cap）；BDD 场景真按键
      断言召回文本 = 通道最新条目；`just test` + `just test-tui` 全绿。
      [blocked-by: T2]
