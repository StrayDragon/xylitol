# status

idle：**1 行空白**呼吸距，无 spinner / Ready。busy：`spinner + 短词` 紧贴左。

- 短词：`Working` / `Running {name}` / `Reloading…` / `Compacting`。禁止 Ready、turn 计数、耗时%、队列徽章进本行。
- Reloading 右侧不画 mcp pending。
- spinner 动画帧序与节奏以包 Loader 实现为准。本稿 `/tui/status/busy?play=1` 对照帧，不是 host 时钟。
- Working **MUST NOT** 进 footer。
