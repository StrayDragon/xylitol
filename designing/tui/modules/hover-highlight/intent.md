# hover-highlight

指针悬停时，用引擎 dim 标出「哪里能点」，不改内容、不触发动作、不写会话。

## MUST

1. 悬停只 dim **块头全宽**、**展开后可见左边轨**、**Ctrl+O 提示带**。MUST NOT dim 工具/bash/diff 输出正文，MUST NOT 整块淡底，MUST NOT 满屏闪。
2. 点击区可以大于悬停区：展开叶块空选点正文仍可折，但悬停不把正文刷暗。
3. 正在拖选时 MUST NOT 画悬停 dim——选区反色优先。
4. 收起叶块：悬停块头。展开叶块、指针在正文：只亮左边轨（正文保持原色）。ActivityFold 信封/簇：只悬停块头行。hint 带：只悬停该提示行。
5. Compaction 仍只三角列可点；悬停几何跟可点三角走，summary 不 dim。待办栏不在本模块。
6. 悬停目标未变 MUST NOT 整段重绘组件树；目标切换 MAY 廉价重投影。
