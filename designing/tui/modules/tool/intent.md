# tool

tool / bash / diff：**1-cell 轨 + gutter + 内容**（与 expandable 同源）。

- 轨色：pending `accent` / 成功 `success` / 失败 `error`。
- **MUST NOT** 默认整行 `tool-*-bg` 洗底；**MUST NOT** ASCII `|` 当轨。
- 折叠摘要仍可读命令/路径；展开详情 **MUST NOT** 再 echo 同一命令。
