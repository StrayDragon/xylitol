# language: zh-CN
# capability: app-tui-input
# purpose: Editor 操作区、取消/退出键位、steer 与 follow-up、信任后 yolo。
# scope: 产品 TUI 面

功能: app-tui-input

  @req:ati1 @human
  场景: editor-operation-zone
    - 输入区 MUST 使用 xylitol-tui Editor，并保留上下边框作为操作区边界；选择器打开时 MUST 替换 editor 槽而非画到内容顶部。

  @req:ati2 @human
  场景: cancel-and-ctrl-c
    - 流式、agent 忙碌或 bang（!/!!）忙碌且无 overlay 时，Esc 与 Ctrl+C（app.clear）MUST 同语义调用 Driver::abort（或等价 pending abort latch）且 MUST NOT 退出 TUI；有 overlay 时 Ctrl+C MUST 先关槽且 MUST NOT abort。idle 且无 overlay 时：编辑器非空 Ctrl+C MUST 清空输入；编辑器为空 Ctrl+C MUST 退出 TUI。

  @req:ati3 @human
  场景: steer-and-followup-keys
    - agent 忙碌时普通 Enter MUST 将当前输入作为 steer 入队；Alt+Enter MUST 将当前输入作为 follow-up 入队（等待当前 agent loop 全部完成后再执行）。

  @req:ati4 @human
  场景: yolo-after-trust
    - 在项目已信任前提下，TUI MUST NOT 实现逐工具审批确认流程；工具默认执行；MUST 保留 hook 扩展点供日后追加策略。

  @req:ati5 @human
  场景: double-esc-session-tree
    - 空编辑器下双 Esc（时间窗与 demo 一致）MUST 打开会话树；树打开时 Esc MUST 关闭树并还原 editor 槽。

  @req:ati7 @human
  场景: demo-steer-preserves-turn
    - agent_demo 在 steer 入队时 MUST 将 steer 文本写入 transcript（可带 [steer] 标记）并可挂到会话活树；MUST NOT 调用会清空 scheduled_actions/pending_events 的新一轮 queue_simulated_turn 入口来「顶替」当前忙碌轮。

  @req:ati8 @human
  场景: demo-bash-prefix-border
    - agent_demo 中当编辑器文本 trim 后以 ! 开头时，Editor 操作区边框 MUST 切换为 bash 强调色（对齐 DESIGN success 前景）；去掉 ! 前缀后 MUST 恢复默认 muted 边框。

  @req:ati9 @human
  场景: demo-external-editor-stub
    - agent_demo MUST 将 Ctrl+G 绑定为外部编辑器原型：记录调用并写入系统提示；MUST NOT 在 harness 路径强制 spawn 真实 $EDITOR；MAY 向编辑器追加 stub 标记以演示往返。

  @req:ati10 @human
  场景: product-host-key-wiring
    - 产品 HostSession MUST 在 agent 忙碌且会话树关闭时将普通 Enter 映射为 Driver::steer、Alt+Enter 映射为 Driver::follow_up；忙碌时 Esc MUST 调用 Driver::abort 并 clear_queue(steer=true, follow_up=false)；MUST NOT 在忙碌 Esc 时打开 c491 stub 树。

  @req:ati11 @human
  场景: product-queue-chrome
    - 产品 TUI 在 steer/follow-up 队列非空时 MUST 在 scrollback 与 status 之间以 muted 色渲染 Steering:/Follow-up: 行及 Alt+Up dequeue 提示（对齐 pi pendingMessagesContainer；实现为 queue strip）；MUST NOT 将排队内容写成 scrollback System [steer]/[follow-up] 墙；MUST NOT 在 footer/status 另加队列计数徽章（如 `q:sN|fM`）；Alt+Up MUST 将队列文本还原进 editor 并 clear_queue(steer=true, follow_up=true)。经 attach/Remote 时，入队后的 drain_pending MUST NOT 用空队列深度把已画条文案清掉；校准 MUST 只按 Host 真实深度 FIFO 消退。

  @req:ati12 @human
  场景: product-queue-uplink
    - 当 steer/follow-up 从队列注入 agent history 时，runtime MUST 发射 MessageStart/MessageEnd（role=user，带 message）；产品 bridge MUST 将其提交为 scrollback UiEntry::User（与 idle 提交同形）；queue strip 随 QueueUpdate 消退后用户文本 MUST 仍留在 transcript；MUST NOT 仅在 strip 闪现后丢失。即使用户连续两次提交相同正文（idle 后再 busy 插话，或两轮根提交），transcript MUST 出现两条用户气泡，MUST NOT 因正文相同而吞掉第二次提问。

  @req:ati13 @human
  场景: product-editor-send-history
    - 产品 Host 在 idle 提交、busy steer、busy follow-up 发送用户文本时 MUST 调用 Editor::add_to_history（跳过空串与连续重复）；同一 TUI session 内 MUST 能在 editor 空或已在浏览态时用 ↑/↓ 召回这些文本（委托包 ed05）；启动与切 session 时 MAY 从 session store 只读装载用户发送历史种子（见 ati41），MUST NOT 要求把 editor 浏览态本身做成独立持久化文件。

  @req:ati41 @human
  场景: editor-history-session-seed
    - 纯 new session（含 /session-new）启动时，产品 Host MUST 在当前 cwd 下按 mtime 取最近 N 个其它已持久化 session（N 来自配置 tui.editor_history_seed_sessions，缺省 1），抽取其中 user 角色正文（跳过 trim 后以 / 开头的行），按较旧 session 先、会话内时间序调用 add_to_history，使 ↑ 先召回全局最近一条；恢复或切换到已有 session 时 MUST 仅用该 session 的 user 正文替换 editor 发送历史缓冲；MUST NOT 写入 assistant/tool/thinking；MUST NOT 改 transcript 或会话树；cwd 比较 MUST 与 resume Current scope 的 cwd_matches 同口径。

  @req:ati14 @human
  场景: product-abort-resumable
    - 产品 TUI 在用户 Esc abort 当前流之后 MUST 回到可提交 idle（follow-up restore 语义不变）；MUST NOT 让后续用户输入持续产生粘性 aborted Error 而无法继续对话。

  @req:ati15 @human
  场景: product-bash-prefix-border
    - 产品 TUI 中当编辑器文本 trim 后以 ! 或 !! 开头时，Editor 操作区边框 MUST 切换为 bash 强调色（对齐 DESIGN success 前景）；去掉该前缀后 MUST 恢复当前 thinking level 边框（atc17），MUST NOT 在已接线 thinking 边框后仍用 muted 覆盖。

  @req:ati16 @human
  场景: product-idle-bang-execute
    - 产品 idle 且会话树关闭时，Enter 提交以 ! 或 !! 开头的非空命令 MUST 调用 Driver::execute_bash（经 Command::Bash / dispatch），MUST NOT 调用 Driver::run；!! 前缀 MUST 设置 exclude_from_context=true，单 ! 为 false；命令体为空时 MUST 提示且 MUST NOT 执行。

  @req:ati17 @human
  场景: product-external-editor
    - 产品 Host MUST 将 Ctrl+G 绑定为外部编辑器入口：交互 TTY 且已配置非空 $VISUAL 或 $EDITOR 时 MUST 经 xylitol_tui::TUI::with_terminal_suspended（或等价）挂起终端、写入 tempfile、spawn 该编辑器、成功退出后把文件内容写回 Editor；harness / 非 TTY / 测试路径 MUST NOT spawn 真实编辑器（保持 stub：系统提示与可选 stub 标记）；未配置 $VISUAL 与 $EDITOR、tempfile/spawn/读回失败、或编辑器非零退出时 MUST NOT panic，MUST 向 scrollback 追加 UiEntry::Error 短行并保留原 Editor 文本；MUST NOT 静默默认 nano/notepad；MUST NOT 在 packages/xylitol-tui 内实现 $EDITOR/tempfile/spawn（包仅提供 with_terminal_suspended）。

  @req:ati18 @human
  场景: editor-slot-machine-live-tree
    - 产品 TUI MUST 以显式 EditorSlot（至少含 Editor 与 Tree；MAY 含 Plate/Settings/Choice）互斥替换 editor 槽；打开任一非 Editor 槽时 MUST 替换贴底 editor 区而非画到 scrollback 顶部；Esc MUST 优先关闭当前槽并还原 Editor；忙碌时 Esc MUST 仍 abort（ati10）且 MUST NOT 打开 Tree；Tree 槽 MUST 由 Driver MessageHistory 活树驱动（见 app-tui-session-tree），MUST NOT 再冻结为仅假树 stub。

  @req:ati19 @human
  场景: bang-esc-aborts
    - 产品 TUI 在交互 !/!! bash 执行期间 MUST 视为忙碌：Esc MUST 调用 Driver::abort 以取消该 bash（进程树按 c660）；host MUST NOT 在 drain_pending 内独占 await bash 以致无法读键盘；取消后 MUST 回到可输入 idle。

  @req:ati20 @human
  场景: bang-busy-hard-reject
    - 产品 TUI 在交互 bang 仍 busy（bash_active）时，用户再次 Enter 提交 ! 或 !! 命令 MUST 硬拒绝：MUST 向用户给出简短提示、MUST 恢复编辑器文本（或等价保留命令体）、MUST NOT 调用第二次 Driver::execute_bash、MUST NOT 排队第二 bang；忙碌时非 bang 前缀文本 MUST 仍按既有 steer 规则处理。

  @req:ati21 @human
  场景: model-picker-slot
    - 产品 TUI 无参 /model MUST 以 EditorSlot（Models 或等价）替换贴底 editor 区展示可用模型 SelectList；支持过滤（fuzzy 或包 API）；↑↓ 选模型；可调模型 ←→ 与槽内 Shift+Tab 在焦点模型声明的 thinking_levels 上选档（wide 铺档名 / narrow 单档标签 / 无思考或仅不可调 off 显示 —）；Enter 选定 → SetModel + thinking（可调默认配置列表末项，不可调为 off）并关槽还原 editor；Esc 取消关槽且 MUST NOT 改模型或 thinking；busy 时 MUST 仍可打开列表（与 atm1/atm16 Allow 一致；选定仍走既有 NextTurn 语义）；MUST NOT 居中 overlay 主路径；MUST NOT 在产品全局键位注册 thinking cycle；MUST NOT 把未在该模型 thinking_levels 声明的档名当成可选项。

  @req:ati23 @human
  场景: model-arg-completion
    - 产品 Editor MUST 经包 CompletionSource 注册 SlashArgCompletionSource（命令 model，默认 MUST NOT with_bare_command）与 SlashCommandSource（至少 exit/model）；键入 /model 加空格（及可选前缀）时 MUST 弹出模型 id 补全（catalog 来自 available_models）；Tab/Enter 选定 MUST 将 id 写入 editor；Esc 关 popup MUST NOT 调用 SetModel；无参 /model Enter 仍 MUST 走 c630 OpenModels 槽，MUST NOT 被 arg Source 抢走。

  @req:ati22 @human
  场景: session-tree-filter-keys
    - 产品 TUI 在 EditorSlot::Tree 打开时：Ctrl+D MUST 将 FilterMode 设为 default；Ctrl+T / Ctrl+U / Ctrl+L / Ctrl+A MUST 在对应模式（no-tools / user-only / labeled-only / all）与 default 之间 toggle；Ctrl+O MUST 按 default→no-tools→user-only→labeled-only→all 循环；Ctrl+Shift+O（或等价 cycleBackward）MUST 反向循环（见 ati26）；上述键 MUST 优先于关树时的 thinking（Ctrl+T）与工具视口（Ctrl+O）；增量搜索键入 MUST 交给包 TreeSelector；有搜索串时 Esc MUST 先清搜索再关树（既有 ati5 语义保留）。

  @req:ati24 @human
  场景: session-tree-fold-keys
    - 产品 TUI 在 EditorSlot::Tree 时 MUST 把 ctrl+left、alt+left、ctrl+right、alt+right 交给包 TreeSelector（fold/unfold 或分支跳转）；MUST NOT 在 Editor 槽把这些键误当成 editor 光标词跳；树关时这些键保持既有 Editor/其它语义。

  @req:ati25 @human
  场景: session-tree-fork-key
    - 产品 TUI 在 EditorSlot::Tree 打开时 Shift+F MUST 触发产品会话 fork 流程（见 app-tui-session-tree ast10）；MUST NOT 在 Editor 槽把 Shift+F 误当成普通输入；树关时 Shift+F 保持既有语义（若无则 noop）。

  @req:ati26 @human
  场景: session-tree-cycle-backward
    - 产品 TUI 在 EditorSlot::Tree 打开时 Ctrl+Shift+O（或 KeybindingsManager 解析到的 cycleBackward 等价和弦）MUST 按 all→labeled-only→user-only→no-tools→default 方向循环 FilterMode；MUST 优先于关树时的其它 Ctrl+O 语义；Ctrl+O 正向循环（ati22）保持不变。

  @req:ati27 @human
  场景: session-tree-label-keys
    - 产品 TUI 在 EditorSlot::Tree 时 Shift+L MUST 打开 label 编辑（非向 editor 写入字面 L）；Shift+T MUST 切换 annotation 时间戳显示；label 编辑中 Esc MUST 取消编辑且 MUST NOT 关树；树关时 Shift+L/T 保持既有语义。

  @req:ati28 @human
  场景: slash-session-tree-fork-keys
    - 产品 TUI idle 且 Editor 槽时 Enter 提交 /session-tree 或 /session-fork MUST 走 PendingSlash（非向 agent 当普通 prompt）；树已开时 /session-tree 再提交 MUST NOT 重复破坏既有槽（可 noop 或刷新）；busy 时 MUST NOT 开树或 fork；/tree 与 /fork MUST NOT 再进入上述 PendingSlash 路径。

  @req:ati30 @human
  场景: input-bdd-guardrails
    - 产品 TUI 输入面关键行为 MUST 可由 rstest-bdd 场景固定：忙碌 Esc abort 与迟到 Xy 不复活；bang Esc 取消与 suppress_idle_esc 后第二 bang 仍可 abort；overlay Esc 先关槽且忙碌无 overlay 时 Esc MUST NOT 开树；忙碌 Enter/Alt+Enter/Alt+Up 与 Driver steer/follow_up/clear_queue 契约。实现可复用 HostSession+ScriptedDriver+HostEvent 注入，MUST NOT 为 BDD 另造第二套副作用泵。

  @req:ati31 @human
  场景: abort-latch-suppresses-xy
    - 产品 TUI 在 agent 忙碌且无 overlay 时按 Esc/Ctrl+C MUST 在同一步进内臂装流抑制并保留 pending_abort 供 drain_pending 调用 Driver::abort；在 drain 完成前注入的迟到 Xy MUST NOT 写出新 assistant 正文或清除已 flush 的 abort 语义（含 partial 与脚注）。

  @req:ati42 @human
  场景: abort-keeps-partial
    - 产品 TUI 在 agent 流式中用户 abort（Esc 或 busy Ctrl+C）时 MUST 将已累积的 streaming thinking/text flush 为正式 scrollback 条目，并追加 abort 脚注（Operation aborted 或文档化等价文案）；MUST NOT 清空已上行可见正文而仅保留 System Aborted。Bang abort 仍走 ati19/(cancelled)，本要求不适用。迟到 Xy 抑制仍遵循 ati31。

  @req:ati32 @human
  场景: busy-bang-prefix-policy
    - 产品 TUI 在 agent 忙碌（run_active / phase Busy）且非 bash_active 时，用户 Enter 提交以 ! 或 !! 开头的文本 MUST 硬拒绝（提示 + 保留编辑器文本），MUST NOT 将该字面量作为 Driver::steer 入队；bash_active 时第二 bang 硬拒绝仍遵循 ati20。

  @req:ati29 @human
  场景: session-resume-panel-keys
    - 产品 TUI 在 EditorSlot::SessionResume 打开时：Tab MUST 切换 scope Current/All；Ctrl+S（或键位表等价）MUST 循环 Sort Threaded/Recent/Fuzzy；Ctrl+N MUST 切换 Name All/Named；Ctrl+P MUST 切换 path 显示；Ctrl+U（或键位表 app.session.toggleId）MUST 切换会话行完整 session id 列显隐（默认隐藏）；Ctrl+R MUST 进入选中项 rename（Esc 取消 rename MUST NOT 改名）；Ctrl+D MUST 进入删除确认（Enter 确认 / Esc 取消）；Threaded 下 ctrl/alt+left|right（tui.tree.foldOrUp / unfoldOrDown）MUST 折叠或展开选中父节点的子会话行；上述键 MUST 优先于 Editor 槽同名语义；面板关闭后键位恢复既有语义；搜索键入 MUST 交给面板 filter 而非误提交 prompt。agent/bang busy 时 Enter 选定会话 MUST NOT SwitchSession，MUST 经壳层通告显示 atm10 约定文案（见 atc22），MUST NOT 为此追加 ScrollNotice；idle 行为不变。

  @req:ati33 @human
  场景: at-path-completion
    - 产品 Editor MUST 经包 CompletionSource 注册 AtPathSource，根目录默认为进程 cwd（harness MAY 注入测试根）；用户键入 @（及路径前缀）时 MUST 弹出文件/目录补全；Tab/Enter 选定 MUST 将路径引用写入 editor；Esc 关 popup MUST NOT 改已有非补全文本；提交消息时本变更 MUST NOT 自动把文件内容读入 transcript（路径文本引用即可）。

  @req:ati34 @human
  场景: paste-collapse-submit
    - 产品 TUI 在 idle 提交、busy steer、busy follow-up、remember_editor_send（历史）以及 Ctrl+G 外部编辑器读出时，MUST 使用 Editor::get_expanded_text（或 UiRoot 等价 API）作为发送/写入文本；显示层 MUST 仍可经 get_text 展示 [paste #N …] 占位；MUST NOT 把未展开的 paste marker 作为 prompt/steer/follow-up 交给 Driver。

  @req:ati35 @human
  场景: keybindings-catalog-hot-reload
    - 产品 TUI MUST 将已接线产品动作注册为 app.*（或文档约定的）keybinding id 并经 KeybindingsManager 匹配，MUST NOT 在产品输入路径新增字面和弦硬编码；MUST 在启动时从用户 agent 目录 keybindings.json（若存在）加载覆盖；MUST 提供 reload_keybindings（或等价）在成功时替换当前会话绑定、失败时保留旧绑定并报告诊断；重载 MUST NOT 清空 transcript 或 session 历史。

  @req:ati40 @human
  场景: dollar-skill 补全与高亮
    - 产品 Editor MUST 经 CompletionSource 在键入 $ 时弹出已加载 skill 名补全；Tab/Enter 选定 MUST 写入 $name；用户消息在 scrollback 中 MUST 用 skill-ref 色高亮 $name token；MUST NOT 另开 skill 色块或系统已加载行。

  @req:ati36 @human
  场景: thinking-level-picker-only
    - 产品 TUI MUST NOT 注册全局 app.thinking.cycle，MUST NOT 在 Editor 槽（非 Models）将 Shift+Tab 绑为 thinking cycle。thinking 档变更 MUST 仅经 /model 槽（←→ / 槽内 Shift+Tab）或有参 /model <id>（可调默认声明列表末项，不可调为 off）路径提交。MUST NOT 新增产品 /thinking-level slash；MUST NOT 与 app.thinking.toggle（Ctrl+T 折叠）混淆；MUST NOT 用包 ThinkingBorderLevel::cycle_next 作为产品真源。

  @req:ati37 @human
  场景: paste-image-temp-path
    - 产品 TUI MUST 支持将系统剪贴板图片粘贴进对话：经 Driver 只读缝读取图片字节；成功时 MUST 写入带不可预测名的 tempfile，并向 Editor 插入绝对路径纯文本（对齐 pi；MUST NOT 要求 @ 前缀）；MUST NOT 在 Editor 内嵌大段 base64；MUST NOT 在 idle/busy 提交时把该路径转成 user AgentPart::Image；MUST NOT 从 app/tui reach infra::clipboard 或 infra::image；当剪贴板无图（或读图失败）时 MUST 经 Driver 尝试读取系统剪贴板 UTF-8 文本并插入 Editor；仅当图与文本皆不可用时 MUST 向 scrollback 追加短 Error；MUST NOT panic。

  @req:ati43 @human
  场景: reload-soft-gate-keys
    - 产品 TUI 在 /reload 进行中（ath28）且无 overlay 时：MUST 允许向 Editor 打字与 Ctrl+G 外编；Enter（普通上行、slash、bang）MUST 硬拒绝并经壳层通告 body `reloading — wait`，MUST NOT 调用 Driver::run / execute_bash / 第二次 reload，MUST NOT 入 steer/follow-up；Esc 与 Ctrl+C MUST 请求取消重载且 MUST NOT 退出 TUI、MUST NOT 打开会话树。有 overlay 时 Esc/Ctrl+C MUST 先关槽且 MUST NOT 仅因此取消重载。取消或重载结束后键位恢复既有 idle/agent/bang 规则（ati2/ati10/ati19）。

  @req:ati44 @human
  场景: attach-queue-stats-calibrate
    - 产品 TUI attach 校准队列条深度时 MUST 使用 Host 只读 queue_stats（或语义等价 unary）与下行 QueueUpdate；Remote 默认空深度 MUST NOT 当作已清空。入队瞬间本地条文案 MUST 保持可见，直到 Host 深度下降。

  @req:ati45 @human
  场景: same-text-second-user-visible
    - 产品 TUI 在用户连续提交相同正文时 MUST 让第二次提问在 transcript 中作为独立用户气泡可见（含 busy 插话注入）；MUST NOT 仅因与上一条 UiEntry::User 正文相等而跳过写入。
