# editor-placeholder

editor 空态占位示例句。

- editor 为空且 idle 时显示一行 muted 占位：`Ask anything… "<示例>"`；示例随新会话轮换。
- busy 或已有输入时占位立即消失（见 busy 态）。
- 占位不是对话正文：不进 scrollback、不可选中、不参与提交。
- 示例文案展示产品能力（改代码 / 跑命令），宽度不足按 atoms 截断规则收 `…`。
