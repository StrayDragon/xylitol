# opencode 可见性开关 research（一手摘录）

来源：opencode PR #4882（tool details visibility）、#12171（thinking visibility）；只读摘录。

## tool details visibility（#4882）

- 新增 `tool_details_visibility`，持久化在 `kv.json`；不是每个会话，是客户端全局显示偏好。
- 命令：`Hide tool details` / `Show tool details`。
- 隐藏规则：**成功**的 tool call 可隐藏；**失败**的始终显示；**ask / permission** 始终显示，不得隐藏。
- 实现上把 tool part 的显示从 `part.type === "tool"` 改为按可见性过滤；成功隐藏时不占高度。
- 目标：长会话里减少工具细节噪声，但保留失败与需要人决策的块。

## thinking visibility（#12171）

- 新增 `thinking_visibility` 开关；keybinding 切换；默认可见。
- 与 tool details 独立：thinking 与 tool 是两个子项。

## 其他显示偏好

- timestamps、diffWrapMode 等也是客户端显示偏好；不是会话语义。

## 对本仓设计的映射（c3120 / c3125）

- 不做单一布尔，而做「预设 + 逐子项模式」：
  - `thinking: hidden | header | full`
  - `tool_header: hidden | summary | full`
  - `tool_args: hidden | preview | full`
  - `tool_output: hidden | preview | full`
  - `diff: hidden | preview | full`
  - `user_message: preview | full`
- 预设 quiet / summary / recent / full 给默认；显式配置 / 运行时开关覆盖。
- 失败 / Ask 块不得被 hidden 吃掉：tool_header hidden 只作用于成功/普通活动；Ask 仍可见（沿用 r1364 Ask 可交互、不得折没）。
- 持久化：端侧会话级内存；配置文件是默认值；不写 Host。opencode 是客户端全局 kv；本仓按「端侧纯呈现」原则，不引入 Host 折叠持久化。
