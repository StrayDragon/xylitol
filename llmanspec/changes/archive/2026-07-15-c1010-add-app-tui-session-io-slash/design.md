# Design — c1010-add-app-tui-session-io-slash

## pi 行为调研

源：`interactive-mode.ts` `handleExportCommand` / `handleImportCommand` / `handleCompactCommand`；`slash-commands.ts` 描述。

### `/export`（→ `/session-export`）

- 路径参数：`getPathCommandArgument`（支持 `"..."` / `'...'` 含空格；否则首空白截断）。
- **默认 HTML**：无参或路径**不以** `.jsonl` 结尾 → `exportToHtml`；仅 `.jsonl` 后缀 → `exportToJsonl`。
- 成功/失败：`showStatus` / `showError` 含路径。

**xylitol**：对齐默认 HTML；无参时走 `dispatch` 今日默认 `export.html`。路径解析 MVP：空白切分首段；引号路径 SHOULD 在 tasks 中实现若成本低，否则记 future。旧名 `/export` 无效。

### `/import`（→ `/session-import`）

- 必填 path；否则 usage 错误。
- **确认框**：`Replace current session with …?`；取消则 status「Import cancelled」。
- 成功：`importFromJsonl` + 状态提示；缺 cwd 可再 prompt。

**xylitol**：必填 path；导入前 MUST 用 **editor 槽 Yes/No SelectList** 确认（非 Trust Choice stub）。确认后 `ImportJsonl` → switch/重建 transcript。旧名 `/import` 无效。不做 MissingSessionCwd 完整向导（失败系统行即可）。

### `/compact`（→ `/session-compact`）

- 可选 `customInstructions`（`/compact …` 余串）传给 `session.compact`。
- 错误多经事件面，handler 吞异常。

**xylitol**：`Command::Compact` **无** instructions 字段 → **仅无参** `/session-compact`；带参 MUST 短错误 usage，MUST NOT 静默忽略。busy 拒绝。旧名 `/compact` 无效。

## 执行路径

`parse` → `PendingSlash` → `drain_pending` → `dispatch(Command::*)` → 系统行 / 确认槽。禁止平行词表。

## 与迁移报告的差异修正

报告草案曾写 export 默认 JSONL —— **以 pi 为准改为默认 HTML**。
