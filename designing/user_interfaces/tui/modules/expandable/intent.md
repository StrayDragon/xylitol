# expandable

块级折叠（thinking / tool / diff / ask）。段/簇信封见 activity-fold。

默认皮肤 **rail**（行首细色轨）：tool/bash/diff = 1-cell status 轨 + gutter + 内容；thinking / user / assistant = **flush**（无轨，正文平铺）。**MUST NOT** 默认整行 `tool-*-bg` 洗底。

## MUST（与产品核对）

1. 折叠一行摘要仍可读，**MUST NOT** 只有图标。三角行首：收 `▸`、开 `▾`。
2. 工具主视线：`Name path[:range]`（如 `Read src/main.rs:42-80`）。ToolName 首字母大写 + accent；path on-surface；范围用 skill-ref。cwd 内相对路径。**MUST NOT** `⚙`。
3. 默认 **tools 正文展开**；**Alt+E** 在 hide ↔ unhide 间切换（Edit `display_diff` 服从同一开关）。
4. 正文仍有高度上限；**Ctrl+O** 切满高，与 Alt+E 正交。提示 `ctrl+o to expand`。硬截断时 MUST NOT 再提供 Ctrl+O。
5. 旁注括号完整和弦：流式 `Thinking  (Ctrl+T)`；结束后 `Thought` / `Thought 17s  (Ctrl+T)`；工具 `Read path  (Alt+E)`。`(Ctrl+T)` **只**属于 thinking **块**头。
6. **timeout 预算**（c2435）：仅当模型显式传了 `timeout` 时，命令类工具（bash/grep/find）header 在 `(Alt+E)` 前追加 muted `(timeout {N}s)`（N=钳制后生效秒数）；走工具默认 MUST NOT 显示。静态文本，无倒计时。
7. 轨色：pending accent / 成功 success / 失败 error。
8. Edit/Diff：header + 正文同一轨；正文 **MUST NOT** 叠 `diff-*-bg` 行底。
9. 摘要已含命令时，展开详情 **MUST NOT** 再 echo 同一命令。bash 走人话 stdout；read 走 content；**MUST NOT** 默认甩 machine JSON。
