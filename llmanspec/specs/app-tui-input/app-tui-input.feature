# language: zh-CN
# capability: app-tui-input
# purpose: Editor 操作区、取消/退出键位、steer 与 follow-up、信任后 yolo。
# scope: src/app/tui/

功能: app-tui-input

  @req:r1283 @human
  场景: editor-operation-zone
    - 输入区 MUST 使用 xylitol-tui Editor，并保留上下边框作为操作区边界；选择器打开时 MUST 替换 editor 槽而非画到内容顶部。

  @req:r1294 @human
  场景: cancel-and-ctrl-c
    - 流式、agent 忙碌或 bang（!/!!）忙碌且无 overlay 时，Esc 与 Ctrl+C（app.clear）MUST 同语义调用 Driver::abort（或等价 pending abort latch）且 MUST NOT 退出 TUI；有 overlay 时 Ctrl+C MUST 先关槽且 MUST NOT abort。idle 且无 overlay 时：编辑器非空 Ctrl+C MUST 清空输入；编辑器为空 Ctrl+C MUST 退出 TUI。

  @req:r1305 @human
  场景: steer-and-followup-keys
    - agent 忙碌时普通 Enter MUST 将当前输入作为 steer 入队；Alt+Enter MUST 将当前输入作为 follow-up 入队（等待当前 agent loop 全部完成后再执行）。

  @req:r1314 @human
  场景: yolo-after-trust
    - 在项目已信任前提下，TUI MUST NOT 实现逐工具审批确认流程；工具默认执行；MUST 保留 hook 扩展点供日后追加策略。

  @req:r1321 @human
  场景: double-esc-session-tree
    - 空编辑器下双 Esc（时间窗与 demo 一致）MUST 打开会话树；树打开时 Esc MUST 关闭树并还原 editor 槽。

  @req:r1322 @human
  场景: demo-steer-preserves-turn
    - agent_demo 在 steer 入队时 MUST 将 steer 文本写入 transcript（可带 [steer] 标记）并可挂到会话活树；MUST NOT 调用会清空 scheduled_actions/pending_events 的新一轮 queue_simulated_turn 入口来「顶替」当前忙碌轮。

  @req:r1323 @human
  场景: demo-bash-prefix-border
    - agent_demo 中当编辑器文本 trim 后以 ! 开头时，Editor 操作区边框 MUST 切换为 bash 强调色（对齐 DESIGN success 前景）；去掉 ! 前缀后 MUST 恢复默认 muted 边框。

  @req:r1324 @human
  场景: demo-external-editor-stub
    - agent_demo MUST 将 Ctrl+G 绑定为外部编辑器原型：记录调用并写入系统提示；MUST NOT 在 harness 路径强制 spawn 真实 $EDITOR；MAY 向编辑器追加 stub 标记以演示往返。

  @req:r1284 @human
  场景: product-host-key-wiring
    - 产品 HostSession MUST 在 agent 忙碌且会话树关闭时将普通 Enter 映射为 Command::Steer、Alt+Enter 映射为 Command::FollowUp（经 dispatch）；忙碌时 Esc MUST 调用 Driver::abort 并 Command::ClearQueue(steer=true, follow_up=false)；MUST NOT 在忙碌 Esc 时打开 c491 stub 树。

  @req:r1285 @human
  场景: product-queue-fixed-zone
    - 产品 TUI 在 steer/follow-up 队列非空时 MUST 在 scrollback 与 status 之间以 muted 色渲染 Steering:/Follow-up: 行及 Alt+Up dequeue 提示（对齐 pi pendingMessagesContainer；实现为 queue strip）；MUST NOT 将排队内容写成 scrollback System [steer]/[follow-up] 墙；MUST NOT 在 footer/status 另加队列计数徽章（如 `q:sN|fM`）；Alt+Up MUST 将队列文本还原进 editor 并 clear_queue(steer=true, follow_up=true)。经 attach/Remote 时，入队后的 drain_pending MUST NOT 用空队列深度把已画条文案清掉；校准 MUST 只按 Host 真实深度 FIFO 消退。

  @req:r1286 @human
  场景: product-queue-uplink
    - 当 steer/follow-up 从队列注入 agent history 时，runtime MUST 发射 MessageStart/MessageEnd（role=user，带 message）；产品 bridge MUST 将其提交为 scrollback UiEntry::User（与 idle 提交同形）；queue strip 随 QueueUpdate 消退后用户文本 MUST 仍留在 transcript；MUST NOT 仅在 strip 闪现后丢失。即使用户连续两次提交相同正文（idle 后再 busy 插话，或两轮根提交），transcript MUST 出现两条用户气泡，MUST NOT 因正文相同而吞掉第二次提问。

  @req:r1287 @human
  场景: product-editor-send-history
    - 产品 Host 在 idle 提交、busy steer、busy follow-up 发送用户文本时 MUST 调用 Editor::add_to_history（跳过空串与连续重复）；同一 TUI session 内 MUST 能在 editor 空或已在浏览态时用 ↑/↓ 召回这些文本（委托包 ed05）；启动与切 session 时 MAY 从 session store 只读装载用户发送历史种子（见 ati41），MUST NOT 要求把 editor 浏览态本身做成独立持久化文件。

  @req:r1316 @human
  场景: editor-history-session-seed
    - 纯 new session（含 /session-new）启动时，产品 Host MUST 在当前 cwd 下按 mtime 取最近 N 个其它已持久化 session（N 来自配置 tui.editor_history_seed_sessions，缺省 1），抽取其中 user 角色正文（跳过 trim 后以 / 开头的行），按较旧 session 先、会话内时间序调用 add_to_history，使 ↑ 先召回全局最近一条；恢复或切换到已有 session 时 MUST 仅用该 session 的 user 正文替换 editor 发送历史缓冲；MUST NOT 写入 assistant/tool/thinking；MUST NOT 改 transcript 或会话树；cwd 比较 MUST 与 resume Current scope 的 cwd_matches 同口径。

  @req:r1288 @human
  场景: product-abort-resumable
    - 产品 TUI 在用户 Esc abort 当前流之后 MUST 回到可提交 idle（follow-up restore 语义不变）；MUST NOT 让后续用户输入持续产生粘性 aborted Error 而无法继续对话。

  @req:r1289 @human
  场景: product-bash-prefix-border
    - 产品 TUI 中当编辑器文本 trim 后以 ! 或 !! 开头时，Editor 操作区边框 MUST 切换为 bash 强调色（对齐 DESIGN success 前景）；去掉该前缀后 MUST 恢复当前 thinking level 边框（atc17），MUST NOT 在已接线 thinking 边框后仍用 muted 覆盖。

  @req:r1290 @human
  场景: product-idle-bang-execute
    - 产品 idle 且会话树关闭时，Enter 提交以 ! 或 !! 开头的非空命令 MUST 经 dispatch 调用 Command::Bash，MUST NOT 调用 Driver::run；!! 前缀 MUST 设置 exclude_from_context=true，单 ! 为 false；命令体为空时 MUST 提示且 MUST NOT 执行。

  @req:r1291 @human
  场景: product-external-editor
    - 产品 Host MUST 将 Ctrl+G 绑定为外部编辑器入口：交互 TTY 且已配置非空 $VISUAL 或 $EDITOR 时 MUST 经 xylitol_tui::TUI::with_terminal_suspended（或等价）挂起终端、写入 tempfile、spawn 该编辑器、成功退出后把文件内容写回 Editor；harness / 非 TTY / 测试路径 MUST NOT spawn 真实编辑器（保持 stub：系统提示与可选 stub 标记）；未配置 $VISUAL 与 $EDITOR、tempfile/spawn/读回失败、或编辑器非零退出时 MUST NOT panic，MUST 向 scrollback 追加 UiEntry::Error 短行并保留原 Editor 文本；MUST NOT 静默默认 nano/notepad；MUST NOT 在 packages/xylitol-tui 内实现 $EDITOR/tempfile/spawn（包仅提供 with_terminal_suspended）。

  @req:r1292 @human
  场景: editor-slot-machine-live-tree
    - 产品 TUI MUST 以显式 EditorSlot（至少含 Editor 与 Tree；MAY 含 Plate/Settings/Choice）互斥替换 editor 槽；打开任一非 Editor 槽时 MUST 替换贴底 editor 区而非画到 scrollback 顶部；Esc MUST 优先关闭当前槽并还原 Editor；忙碌时 Esc MUST 仍 abort（ati10）且 MUST NOT 打开 Tree；Tree 槽 MUST 由 Driver MessageHistory 活树驱动（见 app-tui-session-tree），MUST NOT 再冻结为仅假树 stub。

  @req:r1293 @human
  场景: bang-esc-aborts
    - 产品 TUI 在交互 !/!! bash 执行期间 MUST 视为忙碌：Esc MUST 调用 Driver::abort 以取消该 bash（进程树按 c660）；host MUST NOT 在 drain_pending 内独占 await bash 以致无法读键盘；取消后 MUST 回到可输入 idle。

  @req:r1295 @human
  场景: bang-busy-hard-reject
    - 产品 TUI 在交互 bang 仍 busy（bash_active）时，用户再次 Enter 提交 ! 或 !! 命令 MUST 硬拒绝：MUST 向用户给出简短提示、MUST 恢复编辑器文本（或等价保留命令体）、MUST NOT 调用第二次 Command::Bash、MUST NOT 排队第二 bang；忙碌时非 bang 前缀文本 MUST 仍按既有 steer 规则处理。

  @req:r1296 @human
  场景: model-picker-slot
    - 产品 TUI 无参 /model MUST 以 EditorSlot（Models 或等价）替换贴底 editor 区展示可用模型 SelectList；支持过滤（fuzzy 或包 API）；↑↓ 选模型；可调模型 ←→ 与槽内 Shift+Tab 在焦点模型声明的 thinking_levels 上选档（wide 铺档名 / narrow 单档标签 / 无思考或仅不可调 off 显示 —）；Enter 选定 → SetModel + thinking（可调默认配置列表末项，不可调为 off）并关槽还原 editor；Esc 取消关槽且 MUST NOT 改模型或 thinking；busy 时 MUST 仍可打开列表（与 atm1/atm16 Allow 一致；选定仍走既有 NextTurn 语义）；MUST NOT 居中 overlay 主路径；MUST NOT 在产品全局键位注册 thinking cycle；MUST NOT 把未在该模型 thinking_levels 声明的档名当成可选项。

  @req:r1298 @human
  场景: model-arg-completion
    - 产品 Editor MUST 经包 CompletionSource 注册 SlashArgCompletionSource（命令 model，默认 MUST NOT with_bare_command）与 SlashCommandSource（至少 exit/model）；键入 /model 加空格（及可选前缀）时 MUST 弹出模型 id 补全（catalog 来自 available_models）；Tab/Enter 选定 MUST 将 id 写入 editor；Esc 关 popup MUST NOT 调用 SetModel；无参 /model Enter 仍 MUST 走 c630 OpenModels 槽，MUST NOT 被 arg Source 抢走。

  @req:r1297 @human
  场景: session-tree-filter-keys
    - 产品 TUI 在 EditorSlot::Tree 打开时：Ctrl+D MUST 将 FilterMode 设为 default；Ctrl+T / Ctrl+U / Ctrl+L / Ctrl+A MUST 在对应模式（no-tools / user-only / labeled-only / all）与 default 之间 toggle；Ctrl+O MUST 按 default→no-tools→user-only→labeled-only→all 循环；Ctrl+Shift+O（或等价 cycleBackward）MUST 反向循环（见 ati26）；上述键 MUST 优先于关树时的 thinking（Ctrl+T）与工具视口（Ctrl+O）；增量搜索键入 MUST 交给包 TreeSelector；有搜索串时 Esc MUST 先清搜索再关树（既有 ati5 语义保留）。

  @req:r1299 @human
  场景: session-tree-fold-keys
    - 产品 TUI 在 EditorSlot::Tree 时 MUST 把 ctrl+left、alt+left、ctrl+right、alt+right 交给包 TreeSelector（fold/unfold 或分支跳转）；MUST NOT 在 Editor 槽把这些键误当成 editor 光标词跳；树关时这些键保持既有 Editor/其它语义。

  @req:r1300 @human
  场景: session-tree-fork-key
    - 产品 TUI 在 EditorSlot::Tree 打开时 Shift+F MUST 触发产品会话 fork 流程（见 app-tui-session-tree ast10）；MUST NOT 在 Editor 槽把 Shift+F 误当成普通输入；树关时 Shift+F 保持既有语义（若无则 noop）。

  @req:r1301 @human
  场景: session-tree-cycle-backward
    - 产品 TUI 在 EditorSlot::Tree 打开时 Ctrl+Shift+O（或 KeybindingsManager 解析到的 cycleBackward 等价和弦）MUST 按 all→labeled-only→user-only→no-tools→default 方向循环 FilterMode；MUST 优先于关树时的其它 Ctrl+O 语义；Ctrl+O 正向循环（ati22）保持不变。

  @req:r1302 @human
  场景: session-tree-label-keys
    - 产品 TUI 在 EditorSlot::Tree 时 Shift+L MUST 打开 label 编辑（非向 editor 写入字面 L）；Shift+T MUST 切换 annotation 时间戳显示；label 编辑中 Esc MUST 取消编辑且 MUST NOT 关树；树关时 Shift+L/T 保持既有语义。

  @req:r1303 @human
  场景: slash-session-tree-fork-keys
    - 产品 TUI idle 且 Editor 槽时 Enter 提交 /session-tree 或 /session-fork MUST 走 PendingSlash（非向 agent 当普通 prompt）；树已开时 /session-tree 再提交 MUST NOT 重复破坏既有槽（可 noop 或刷新）；busy 时 MUST NOT 开树或 fork；/tree 与 /fork MUST NOT 再进入上述 PendingSlash 路径。

  @req:r1306 @human
  场景: input-bdd-guardrails
    - 产品 TUI 输入面关键行为 MUST 可由 rstest-bdd 场景固定：忙碌 Esc abort 与迟到 Xy 不复活；bang Esc 取消与 suppress_idle_esc 后第二 bang 仍可 abort；overlay Esc 先关槽且忙碌无 overlay 时 Esc MUST NOT 开树；忙碌 Enter/Alt+Enter/Alt+Up 与 Command::Steer/FollowUp/ClearQueue 契约。实现可复用 HostSession+ScriptedDriver+HostEvent 注入，MUST NOT 为 BDD 另造第二套副作用泵。

  @req:r1307 @human
  场景: abort-latch-suppresses-xy
    - 产品 TUI 在 agent 忙碌且无 overlay 时按 Esc/Ctrl+C MUST 在同一步进内臂装流抑制并保留 pending_abort 供 drain_pending 调用 Driver::abort；在 drain 完成前注入的迟到 Xy MUST NOT 写出新 assistant 正文或清除已 flush 的 abort 语义（含 partial 与脚注）。

  @req:r1317 @human
  场景: abort-keeps-partial
    - 产品 TUI 在 agent 流式中用户 abort（Esc 或 busy Ctrl+C）时 MUST 将已累积的 streaming thinking/text flush 为正式 scrollback 条目，并追加 abort 脚注（Operation aborted 或文档化等价文案）；MUST NOT 清空已上行可见正文而仅保留 System Aborted。Bang abort 仍走 ati19/(cancelled)，本要求不适用。迟到 Xy 抑制仍遵循 ati31。

  @req:r1308 @human
  场景: busy-bang-prefix-policy
    - 产品 TUI 在 agent 忙碌（run_active / phase Busy）且非 bash_active 时，用户 Enter 提交以 ! 或 !! 开头的文本 MUST 硬拒绝（提示 + 保留编辑器文本），MUST NOT 将该字面量作为 Command::Steer 入队；bash_active 时第二 bang 硬拒绝仍遵循 ati20。

  @req:r1304 @human
  场景: session-resume-panel-keys
    - 产品 TUI 在 EditorSlot::SessionResume 打开时：Tab MUST 切换 scope Current/All；Ctrl+S（或键位表等价）MUST 循环 Sort Threaded/Recent/Fuzzy；Ctrl+N MUST 切换 Name All/Named；Ctrl+P MUST 切换 path 显示；Ctrl+U（或键位表 app.session.toggleId）MUST 切换会话行完整 session id 列显隐（默认隐藏）；Ctrl+R MUST 进入选中项 rename（Esc 取消 rename MUST NOT 改名）；Ctrl+D MUST 进入删除确认（Enter 确认 / Esc 取消）；Threaded 下 ctrl/alt+left|right（tui.tree.foldOrUp / unfoldOrDown）MUST 折叠或展开选中父节点的子会话行；上述键 MUST 优先于 Editor 槽同名语义；面板关闭后键位恢复既有语义；搜索键入 MUST 交给面板 filter 而非误提交 prompt。agent/bang busy 时 Enter 选定会话 MUST NOT SwitchSession，MUST 经通知条显示 atm10 约定文案（见 atc22），MUST NOT 为此追加 ScrollNotice；idle 行为不变。

  @req:r1309 @human
  场景: at-path-completion
    - 产品 Editor MUST 经包 CompletionSource 注册 AtPathSource，根目录默认为进程 cwd（harness MAY 注入测试根）；用户键入 @（及路径前缀）时 MUST 弹出文件/目录补全；Tab/Enter 选定 MUST 将路径引用写入 editor；Esc 关 popup MUST NOT 改已有非补全文本；提交消息时本变更 MUST NOT 自动把文件内容读入 transcript（路径文本引用即可）。

  @req:r1310 @human
  场景: paste-collapse-submit
    - 产品 TUI 在 idle 提交、busy steer、busy follow-up、remember_editor_send（历史）以及 Ctrl+G 外部编辑器读出时，MUST 使用 Editor::get_expanded_text（或 UiRoot 等价 API）作为发送/写入文本；显示层 MUST 仍可经 get_text 展示 [paste #N …] 占位；MUST NOT 把未展开的 paste marker 作为 prompt/steer/follow-up 交给 Driver。

  @req:r1311 @human
  场景: keybindings-catalog-hot-reload
    - 产品 TUI MUST 将已接线产品动作注册为 app.*（或文档约定的）keybinding id 并经 KeybindingsManager 匹配，MUST NOT 在产品输入路径新增字面和弦硬编码；MUST 在启动时从用户 agent 目录 keybindings.json（若存在）加载覆盖；MUST 提供 reload_keybindings（或等价）在成功时替换当前会话绑定、失败时保留旧绑定并报告诊断；重载 MUST NOT 清空 transcript 或 session 历史。

  @req:r1315 @human
  场景: dollar-skill 补全与高亮
    - 产品 Editor MUST 经 CompletionSource 在键入 $ 时弹出已加载 skill 名补全；Tab/Enter 选定 MUST 写入 $name；用户消息在 scrollback 中 MUST 用 skill-ref 色高亮 $name token；MUST NOT 另开 skill 色块或系统已加载行。

  @req:r1312 @human
  场景: thinking-level-picker-only
    - 产品 TUI MUST NOT 注册全局 app.thinking.cycle，MUST NOT 在 Editor 槽（非 Models）将 Shift+Tab 绑为 thinking cycle。thinking 档变更 MUST 仅经 /model 槽（←→ / 槽内 Shift+Tab）或有参 /model <id>（可调默认声明列表末项，不可调为 off）路径提交。MUST NOT 新增产品 /thinking-level slash；MUST NOT 与 app.thinking.toggle（Ctrl+T 折叠）混淆；MUST NOT 用包 ThinkingBorderLevel::cycle_next 作为产品真源。

  @req:r1313 @human
  场景: paste-image-temp-path
    - 产品 TUI MUST 支持将系统剪贴板图片粘贴进对话：经 Driver 只读缝读取图片字节；成功时 MUST 写入带不可预测名的 tempfile，并向 Editor 插入绝对路径纯文本（对齐 pi；MUST NOT 要求 @ 前缀）；MUST NOT 在 Editor 内嵌大段 base64；MUST NOT 在 idle/busy 提交时把该路径转成 user AgentPart::Image；MUST NOT 从 app/tui reach infra::clipboard 或 infra::image；当剪贴板无图（或读图失败）时 MUST 经 Driver 尝试读取系统剪贴板 UTF-8 文本并插入 Editor；仅当图与文本皆不可用时 MUST 向 scrollback 追加短 Error；MUST NOT panic。

  @req:r1318 @human
  场景: reload-soft-gate-keys
    - 产品 TUI 在 /reload 进行中（ath28）且无 overlay 时：MUST 允许向 Editor 打字与 Ctrl+G 外编；Enter（普通上行、slash、bang）MUST 硬拒绝并经通知条 body `reloading — wait`，MUST NOT 调用 Driver::run / Command::Bash / 第二次 Reload，MUST NOT 入 Command::Steer/FollowUp；Esc 与 Ctrl+C MUST 请求取消重载且 MUST NOT 退出 TUI、MUST NOT 打开会话树。有 overlay 时 Esc/Ctrl+C MUST 先关槽且 MUST NOT 仅因此取消重载。取消或重载结束后键位恢复既有 idle/agent/bang 规则（ati2/ati10/ati19）。

  @req:r1319 @human
  场景: attach-queue-stats-calibrate
    - 产品 TUI attach 校准队列条深度时 MUST 使用 Host 只读 queue_stats（或语义等价 unary）与下行 QueueUpdate；Remote 默认空深度 MUST NOT 当作已清空。入队瞬间本地条文案 MUST 保持可见，直到 Host 深度下降。

  @req:r1320 @human
  场景: same-text-second-user-visible
    - 产品 TUI 在用户连续提交相同正文时 MUST 让第二次提问在 transcript 中作为独立用户气泡可见（含 busy 插话注入）；MUST NOT 仅因与上一条 UiEntry::User 正文相等而跳过写入。

  @req:r1297 @executable
  场景: session-tree-filter-keys-headless
    当 打开样例会话树并挂载交互面
    当 在树槽按下和弦 "Ctrl+U"
    那么 树过滤模式为 "user-only"
    并且 思考折叠默认态未被树槽过滤键触碰
    当 在树槽按下和弦 "Ctrl+A"
    那么 树过滤模式为 "all"

  @req:r1299 @executable
  场景: session-tree-fold-keys-headless
    当 打开样例会话树并挂载交互面
    当 选中节点 "p1" 再收到和弦 "Ctrl+Left"
    那么 该节点子会话行被收起
    当 选中节点 "p1" 再收到和弦 "Alt+Right"
    那么 该节点子会话行重新展开

  @req:r1300 @executable
  场景: session-tree-fork-key-headless
    当 打开样例会话树并挂载交互面
    当 选中节点 "root" 再收到和弦 "Shift+F"
    那么 fork 请求交给主机且编辑器未收到字面输入

  @req:r1301 @executable
  场景: session-tree-cycle-backward-key-headless
    当 打开样例会话树并挂载交互面
    当 在树槽按下和弦 "Ctrl+Shift+O"
    那么 树过滤模式为 "all"
    当 在树槽按下和弦 "Ctrl+Shift+O"
    那么 树过滤模式为 "labeled-only"

  @req:r1302 @executable
  场景: session-tree-label-keys-headless
    当 打开样例会话树并挂载交互面
    当 选中节点 "root" 再收到和弦 "Shift+L"
    那么 标签编辑在树内打开且编辑器未收到字面输入
    当 在树槽按下和弦 "Esc"
    那么 树仍开着且无标签写入动作排入

  @req:r1312 @executable
  场景: thinking-cycle-not-bound-editor-slot-headless
    当 挂载空模型的编辑器交互面
    当 在编辑器槽按下和弦 "Shift+Tab"
    那么 编辑器文本保持为空且仍在编辑器槽
    当 在编辑器槽按下和弦 "Ctrl+T"
    那么 思考折叠默认态翻转且全帧不出现模型列表

  @req:r1294 @executable
  场景: busy-abort-and-idle-quit-keys-headless
    当 以主机泵开启忙碌流并按下 Esc
    那么 abort 计一次且未退出且回到可输入 idle
    当 再次开启忙碌流并按下 Ctrl+C
    那么 abort 同语义计两次且未退出
    当 打开样例树槽后按下 Ctrl+C
    那么 树槽关闭且未退出且未新增 abort
    当 编辑器输入草稿后按下 Ctrl+C
    那么 编辑器被清空且未退出且未新增 abort
    当 清空编辑器再按下 Ctrl+C
    那么 会话请求退出

  @req:r1305 @executable
  场景: busy-enter-steer-alt-enter-followup-headless
    当 以主机泵开启忙碌流并输入 nudge 后按 Enter
    那么 steer 收到 nudge 且未发起第二次 run
    当 输入 later 并按 Alt+Enter
    那么 follow_up 收到 later 且未发起第二次 run

  @req:r1284 @executable
  场景: busy-esc-aborts-clears-steer-no-tree-headless
    当 以主机泵开启忙碌流并按下 Esc
    那么 abort 计一次且清队为 steer 不清 follow_up
    并且 会话树未打开

  @req:r1288 @executable
  场景: abort-then-resubmit-runs-again-headless
    当 以主机泵开启忙碌流并按下 Esc 后收流关闭
    那么 run 不再活跃且 abort 已计一次
    当 输入 second 并按 Enter
    那么 run 收到 second 且无粘性 aborted 错误

  @req:r1290 @executable
  场景: idle-bang-execute-not-run-headless
    当 以主机泵在 idle 提交 bang 命令
    那么 execute_bash 收到命令体且未调用 run
    当 提交双感叹号命令
    那么 exclude_from_context 为真
    当 提交空命令体的感叹号
    那么 有提示且未执行 bash 且未调用 run

  @req:r1293 @executable
  场景: bang-esc-aborts-hanging-bash-headless
    当 以主机泵提交挂起 bang 并经输入流注入 Esc
    那么 abort 到达驱动且 Bash 块为 cancelled 且回到 idle
    并且 无 agent 的 Aborted 滚动提示

  @req:r1295 @executable
  场景: bash-active-second-bang-hard-reject-headless
    当 以主机泵令 bash 执行中并提交第二条 bang
    那么 有硬拒提示且编辑器保留命令体
    并且 未发起第二次 execute_bash 且未排队

  @req:r1303 @executable
  场景: slash-session-tree-pending-not-prompt-headless
    当 以主机泵在 idle 提交斜杠 session-tree
    那么 会话树打开且未作为 prompt 调用 run
    当 开启忙碌流再提交斜杠 session-tree
    那么 树未重复打开
    当 在 idle 提交斜杠 tree
    那么 无该动词路径且未开树且未调用 run

  @req:r1306 @executable
  场景: bang-esc-then-second-bang-still-abortable-headless
    当 以主机泵提交挂起 bang 并注入 Esc 加积压 Esc
    当 再提交第二条挂起 bang 并注入 Esc
    那么 第二次 bang 仍可被 Esc 中止且块为 cancelled

  @req:r1307 @executable
  场景: abort-latch-suppresses-late-xy-headless
    当 以主机泵开启忙碌流注入正文增量后按下 Esc 再注入迟到增量
    那么 已流式正文保留且迟到增量不出现
    并且 abort 计一次且滚动提示为 aborted 语义

  @req:r1308 @executable
  场景: agent-busy-bang-prefix-hard-reject-headless
    当 以主机泵开启忙碌流并提交 bang 前缀文本
    那么 有硬拒提示且编辑器保留文本
    并且 未入 steer 且未调用 execute_bash

  @req:r1318 @executable
  场景: reload-soft-gate-keys-headless
    当 以主机泵开启重载并输入草稿后按 Enter
    那么 通知条为 reloading 且草稿保留且未新增滚动提示
    当 经输入流注入 Esc 取消挂起重载
    那么 重载结束且通告为已取消
    当 重载结束后提交 bang 命令
    那么 键位恢复 idle 规则且 execute_bash 收到命令体
