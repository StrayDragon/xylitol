# Design: c1320 tool path stream chrome

## Pi SSOT（已调研）

| 工具 | Header 格式 | 行信息 |
|---|---|---|
| write | `write {path\|…}` + 正文 viewport | 仅 `(N more lines…)` 截断提示，无行域 |
| edit | `edit {path\|…}` | **仅** diff 正文行号；header **禁止** `:N` |
| read | `read {path\|…}{:start}{-end?}` | `offset` 默认起点 1；`end = offset+limit-1` |

路径 helper：`file_path ?? path`；空 → muted `…`；`$HOME` → `~/`。

## 产品缺口

1. 流式阶段 `human_tool_args_preview` 在缺 path 时返回裸 `edit` / `write (N lines)`，无 `…` 占位。
2. upsert 每次整替换 `args_preview`，partial 解析若短暂丢 path，header 回退。
3. ToolExecutionEnd 不从 result JSON 回填 path 到 preview。
4. read 忽略 `offset`/`limit`。
5. 未认 `file_path` 别名（pi 兼容）。

## Decisions

### A. Preview 格式（`preview.rs`）

```
write {short|…} [(N lines)]
edit {short|…}
read {short|…}[:start[-end]]
```

- keys：`path` | `file_path` | `file`（及既有 edits[0].path）
- 空 path 字符串 → 显示 `…`（不是省略整个 path 槽）

### B. 粘性 path（upsert）

`upsert_tool_entry`：若新 preview 无真实 path（仅 `…` 或工具名），且旧 `args_preview` 已含缩短 path，则保留旧 path 片段再合成（或存 `UiEntry::Tool.path: Option<String>` 字段——优先专用字段，避免从 preview 反解析）。

推荐：`UiEntry::Tool` 增 `tool_path: Option<String>`，preview 只读该字段 + 行数/range；渲染仍走 `args_preview` 字符串（少改 scrollback）。

### C. End 回填（修订）

`ToolExecutionEnd`：**不得**用空 synthetic 重写整段 `args_preview`（会抹掉 bash `$ cmd` 与已流式 path——对齐 pi：`updateResult` 只改结果/ tint，call header 仍来自 streaming args）。

仅当 header 仍缺真实 path（`edit ...` / 裸名）且 result JSON 含 path 时，才回填 `tool_path` 并刷新 preview。

### D. edit 行信息

不改 header。确认合块 `display_diff` 默认可见（c1300）；若 diff 未就绪则仅 `edit {path|…}` + pending tint。

## Non-goals

- 改 DESIGN pending 色
- compact read 分类头
- 改 infra edit/write schema 强制 path-before-content
