# tool

tool / bash / diff：**1-cell 轨 + gutter + 内容**（与 expandable 同源）。

- 轨色：pending `accent` / 成功 `success` / 失败 `error`。
- **MUST NOT** 默认整行 `tool-*-bg` 洗底；**MUST NOT** ASCII `|` 当轨。
- 折叠摘要仍可读命令/路径；展开详情 **MUST NOT** 再 echo 同一命令。

todo_* 工具块：

- 折叠摘要 **MUST** 为条目计数形态（如 `5 items · 1 in progress`）；尚未解析到
  条目时 `...` 占位。**MUST NOT** 摘要携带完整条目 JSON。
- 展开正文 **MUST** 为清单形态：每条目一行（状态字形 + 内容），状态字形与
  Todo checklist 投影**共用同一张表**。**MUST NOT** 把调用参数或结果的原始
  JSON 当正文。
- 失败结果 **MUST** 保持错误文本原样可见，**MUST NOT** 套用清单渲染。
- 直播与恢复重建 **MUST** 同形态（同一 helper）；清单行随类型化状态事件刷新，
  **MUST NOT** 依赖端解析结果文本。
