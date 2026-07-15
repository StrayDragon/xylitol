# Design — c1015-add-app-tui-session-panel-slash

## pi 行为调研

### `/session`（同名）

- **不是**操作菜单。`handleSessionCommand` 把 Session Info / Messages / Tokens / Cost 文本块写入 **chatContainer**（scrollback）。
- 无子命令；无参 only。

**xylitol 钉死 Open #1**：仅无参；经 `GetSessionStats`（+ 必要 `GetState`）格式化为系统/scrollback 文本块。**不做**操作列表（修正迁移报告「操作列表」设想；发现性靠 SlashCommandSource 与其它 `/session-*`）。

### `/resume`（→ `/session-resume`）

- 无参 → `showSessionSelector()`：`SessionManager.listAll`，按 **mtime 降序**；可选 rename；选中 → `switchSession`。
- slash 层**无** path 直参。

**xylitol 钉死 Open #2–#3**：

| Open | 决议 |
|---|---|
| #2 列表字段 | Driver seam 列出会话；排序 **mtime 降序**；行展示 **name（若有）+ id/stem**；路径可作 secondary |
| #3 switch 后 | 重建 transcript；关闭树/其它槽；清与旧 session 绑定的 pending UI；对齐既有 `switch_session` 路径 |
| 直参 | MVP **对齐 pi：仅无参开槽**；`/session-resume <id>` 列为 future（不进本 change MUST） |

UI：editor 槽 SelectList（对齐 `/model`），禁止 capturing overlay 主路径。旧名 `/resume` 无效。

## Seam

今日 Driver 无 `list_sessions` → 本 change **最小扩** `Driver::list_sessions`（或等价），TUI MUST NOT reach `infra::session`。是否加 `protocol::Command`：若 server 暂不对称，可仅本地 Driver 方法；升格 tasks 要求论证一句并优先不扩线协议除非 REST 同步需要。

## 非目标

- `/session-new` `/session-clone` `/session-name`；列表内 rename；pi 级 cost/cache 浪费细项（stats 有啥展示啥）。
- c1010 的 compact/export/import 实现（可在 `/session` 文案末行提示命令名，非必须）。
