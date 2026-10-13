# Design：修在 app seam 还是包层？

- **选 app seam 反转**：包层两个历史入口（`replace_history` / `merge_history`）
  统一 oldest→newest 输入约定，包层循环体不动（行为面最小）；storage 的新→旧
  磁盘序在 app 层 seam（`merge_client_history`）一处反转，谁产生新→旧谁负责转换。
- 不选包层内 `rev()`：会把「输入序」语义留在调用方猜，两个调用约定分叉。
- 包层 doc 改契约 + 顺序单测钉住；BDD 场景走既有 app-tui-host harness seam
  （`HostEvent::Input` 真按键 + 渲染/编辑器快照断言），不新增 harness 面。
