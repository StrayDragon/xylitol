# 设计：鼠标优先交互

## 分层归属

| 能力 | 层 | 理由 |
|---|---|---|
| dock 绝对行命中 | xylitol-tui 指针管线 | 命中坐标系是引擎概念；产品 dock 行号由宿主推算 |
| 动作 chip 渲染与列区间 | xylitol-tui 新组件 | 无业务语义，对齐 Tabs::title_col_ranges 既有形态 |
| 列表行指针映射 | xylitol-tui 列表组件 | SelectList / TreeSelector / ChoicePrompt 已知自身行布局，槽不必重算 |
| dock 动作表与 chip 文案 | xylitol-app-tui | 业务动作与措辞属产品 |
| slot 行点击语义（选中 / 二次点击执行） | xylitol-app-tui | 执行后果属产品（fork、切会话、toggle 能力） |

## 1. dock 命中空间

现状：HitRegionList::region_at 先早退 screen_row >= transcript_rows，而 transcript_rows = rows - dock_rows。dock 内登记的 chrome 命中不可达（队列条 QUEUE_STRIP_HIT_ID 当前即如此）。

改法：HitSpace 增 Dock 变体，其 content_row 语义为**终端绝对行**，匹配时不看 transcript_rows，也不受视口滚动影响。Hit 增 at_screen(id, col, row) 构造器（space = Dock）。

- Content / Viewport 的行门不变。
- apply_hover_tint 只处理 transcript 行；dock 悬停本轮不画（Moved 不进 dock 绘制路径），chip 靠括号与反色表达可点。该限制写入规格，避免日后当 bug 修。

## 2. 动作 chip

ActionRow：Vec<ActionChip { label, active, disabled }> → 单行文本 + 每 chip 列区间。

- 渲染形态：[label]，间隔 1 空格；active 反色，disabled 用 muted。
- chip_col_ranges() -> Vec<ChipRange { start, end, index }>，与 Tabs::title_col_ranges 同口径（列相对本组件渲染起点）。
- 宽度不够时整 chip 丢弃（从尾部丢），不拆成半截。

## 3. footer 组合

footer 从单段 Text 改为左正文 + 右 chip。示意：

    ~/proj · model · thinking · used 42k tokens · 32.8%/128k      [详情] [侧栏]

- 右 chip 宽度先从行宽扣除，余下给左侧并按现有规则右截断。
- 档位：标准档（≥80 列）画 [详情] [侧栏]；极窄 / 窄档只画 [详情]（侧栏本就在 <80 不渲染，其 chip 会让宽）；存在 per-id 折叠覆盖或非默认默认态时额外画 [重置折叠]（仅标准档及以上）。
- [侧栏] 在侧栏开启时 active；[详情] 在覆盖弹层开启时 active。
- 点击 [侧栏] 在窄档触发既有降级路径（开覆盖弹层），不是静默失败。

## 4. dock 动作表

每次渲染时 UiRoot 清空并重建 dock_hits: Vec<DockActionHit { row, col_start, col_end, action }>：

- footer chip 行：逐 chip 登记。
- 队列条：整条登成 DequeueQueue（替代 QUEUE_STRIP_HIT_ID 的不可达 chrome 登记）。
- 待办栏翼头：保持既有 last_todo_hits 路径不变。

分派顺序在 UiRoot::handle_pointer_input：dock 动作 → 待办栏 → 未命中。新增动作只需加 DockAction 变体与一条登记。

## 5. 回显去键名

| 现状 | 改为 |
|---|---|
| (Ctrl+T) | 删除；头行保留折叠三角 + 标签 |
| (Alt+E)、(Alt+E to expand)、(Alt+Shift+E) | 删除 |
| ctrl+o to expand / ctrl+o to fold | click to expand / click to fold |
| Running… (Esc) | 保留（Esc 仍是唯一安全网） |
| ↳ Alt+Up to edit all queued messages | ↳ click to edit all queued messages |

expandable_output 的 ExpandableOutputOptions 默认文案是包内默认值，演示面仍用旧文案；产品侧显式传入新文案。

## 6. 槽鼠标语义

统一四件事：

1. **行点击**：列表控件按屏行给出下标；槽把它映射成「选中」或「执行」。
2. **二次点击执行**：点击行时，若该行已是当前选中行则执行主动作，否则只选中。不做双击计时（终端双击不可靠）。
3. **关闭 chip**：槽头右侧画 [关闭]；import 用 [是] [否]；ask 用 [提交] [跳过]。
4. **能力行一步 toggle**：capabilities 行点击直接 toggle（开关可逆）。

树与 resume 的主动作（Enter 等价）保持既有后果：travel / 切会话。fork、标注、重命名、删除仍走键盘（破坏性或难撤销，不给单击入口），对应键位表保留并在帮助行保留提示。

## 7. 待办栏缺省面

todo_doing_open = true，todo_past_open = false，todo_pending_open = false。

翼头行照旧全部绘制（含零计数）；点翼头展开。既有行高上限、滚动、边框 ↑ N more、正文划选不变。

## 8. 测试边界（seam）

| seam | 位置 | 形式 |
|---|---|---|
| 指针管线 dock 命中 | crates/xylitol-tui/src/pointer.rs 单测 | 命中区登记后按绝对行查询 |
| 动作 chip 列区间 | crates/xylitol-tui/src/components/action_row.rs 单测 | 渲染后读区间表 |
| 列表行映射 | 各列表组件单测 | 设 origin 后按屏行取下标 |
| 键位目录 | crates/xylitol-app-tui/src/tests.rs harness | 按 action 直接调用 + 断言旧和弦不再触发 |
| dock 动作点击 | crates/xylitol-app-tui/src/tests.rs 鼠标事件 | 注入 Down/Up 到 chip 行列，断言状态变化 |
| 槽行点击 | 应用端 harness | 注入鼠标事件到列表行 |
| 待办栏缺省面 | todo_bar.rs 单测 | 默认参数渲染断言 |

BDD 场景只加在可程序判定的公共边界：键位目录收缩（app-tui-input）、footer chip 与 dock 点击（app-tui-fixed-zone）、待办栏缺省面（agent-todo）、块头回显（app-tui-transcript）。
