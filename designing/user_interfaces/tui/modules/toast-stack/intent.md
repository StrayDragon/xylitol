# toast-stack

视口右上角 **通知栈**：成功/说明类短时堆叠。不是通知条，不是滚动提示。

- 成功确认（至少 Copied）以 `✓` 起头；说明类（至少整段 Heuristic 估计首次提示）以 `◆` 起头。单格宽，与折叠箭头 `▸▾`、提示符 `❯` 区分。
- 右上角纵向堆叠，最新在上，最多约 3 条；TTL 到期自动消失。不进 transcript / ScrollNotice；不计入下缘 Fixed-Zone Footprint。
- 错误/拒闸仍走通知条（status 上方恰好 1 行，`Error: ` + warning）。
- 本波无栈内键盘动作、无展开侧板。
