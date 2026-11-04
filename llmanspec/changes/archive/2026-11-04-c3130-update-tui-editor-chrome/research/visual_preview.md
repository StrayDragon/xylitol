# Visual preview — TUI editor chrome

一次性预览脚本，不是产品代码。用于确认 c3130 的圆角、spinner、阶段名落位、`◨` 与折叠 chip 落位。

## 运行

```bash
python3.12 llmanspec/changes/c3130-update-tui-editor-chrome/research/visual_preview.py --variant all
python3.12 llmanspec/changes/c3130-update-tui-editor-chrome/research/visual_preview.py --variant rounded-pulse-summary --no-color
```

- 终端输出为 ANSI 着色帧；同时写 `visual_preview.html`（带颜色，可直接用浏览器打开）。
- 变体：rounded / plain；pulse / hex / diamond spinner；summary / quiet / recent / full 折叠等级。
- 定稿变体：`rounded-pulse-summary`（B+E+D）。
- 宽度默认 100；`--width 120` 可看更宽布局。

## 定稿（用户确认）

- **圆角**：`╭ ╮ ╰ ╯`；DESIGN 同步改为「编辑区边框圆角」。
- **阶段名**：上边框右端，右对齐，不居中。
- **侧栏**：`◨` 在输入内容行右端，与 `❯` 同一行；预留 2 列。
- **折叠 chip**：`[▤ 摘要]` 在下边框右端；窄档隐藏。
- **重置 chip**：仅在有覆盖时出现，位于下边框折叠 chip 左侧。
- **spinner**：固定 `⬢`，accent 亮/暗两档颜色脉冲，100ms；不换字形。
- **折叠 chip 文案**：A，`[▤ 摘要]`。

## 定稿帧（rounded / pulse / summary）

```text
⬢ Drafting reply
╭────────────────────────────────────────────────────────── 新功能A ───╮
❯                                                                  ◨
╰──────────────────────────────────────────────────── [▤ 摘要]───╯
~/Projects/xylitol · local-model · high
```

## 对照帧（plain / hex / summary）

```text
⬢ Drafting reply
─────────────────────────────────────────────────────────── 新功能A ────
❯                                                                  ◨
───────────────────────────────────────────────────── [▤ 摘要]────
~/Projects/xylitol · local-model · high
```

## 其他对照

- **hex**：`⬡ ⬢ ⬣ ⬢`，字形轮换；不是定稿。
- **diamond**：`◇ ◆ ◈ ◆`，晶体感更强；不是定稿。
- **pulse**：固定 `⬢`，颜色脉冲；定稿。
- **折叠 chip**：`[▤ 摘要]` / `[▤ 全折]` / `[▤ 最近]` / `[▤ 全展]`；窄档隐藏。
