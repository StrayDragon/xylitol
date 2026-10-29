# delayed-changes / next-todo

2026-08-10 曾在提案头写「已升格 / 排序 #N」，**目录从未迁入** `llmanspec/changes/`。公开预览与 SDD graph 都以本目录仍为 **park** 为准：不要当成待落地队列。

升格 = 把提案拷回 `llmanspec/changes/` 再走 propose → start。归档时删掉这里的副本（一 id 一处真值）。

**Todo 对模型可见**：不升格本目录。`c2805-update-todo-llm-api` 已吸收「出站 generate 尾插当前清单、不暴露 todo_list、不另开栏/refresh 通道」。

## 仍延后

| id | 说明 |
|---|---|
| `c1910-update-compaction-freeze-tool-replacements/` | 压缩大工具输出替换串首次冻结，保前缀一致 |
| `c1915-add-previous-response-id-optional-chain/` | `previous_response_id` 增量续跑，配置开启；与 c2620 相关 |

旧状态栏子系统（c1895–c1898）已移除：现行固定区为单行 status + 队列条 + 待办栏，不再按独立 Lane Runtime 设计。
