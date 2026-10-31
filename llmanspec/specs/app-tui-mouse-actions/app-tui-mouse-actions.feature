# language: zh-CN
# purpose: 鼠标优先交互：常用操作均有指针入口，键盘退为安全网与高级加速器；固定区动作 chip 与槽行点击语义。

功能: app-tui-mouse-actions

  @req:r3031
  规则: pointer-path-for-common-actions
    产品 TUI 的常用操作 必须均有指针入口：思维块展开、工具与 diff 块展开、输出视口展开、活动段展开与收纳、会话详情弹层显隐、会话侧栏显隐与宽度档循环、队列取回、槽内列表选中与执行、槽关闭。键盘绑定 不得为这些操作的唯一入口。安全网动作例外：中断（Esc）、清空与退出（Ctrl+C）、提交（Enter）、外编（Ctrl+G）、图片粘贴（Ctrl+V）必须保留可用键位。

    场景: 所有常用操作均可经指针完成
      假如 产品 TUI 已就绪且存在可展开的思维块与工具块
      当 不按任何组合键而仅用左键与滚轮操作
      那么 思维块与工具块可展开可收纳且详情弹层与侧栏可开关
  @req:r3032
  规则: default-binding-contraction
    以下动作 id 必须保留注册与分发，但默认和弦必须为空：app.thinking.toggle、app.tools.expand、app.tools.blocks、app.activity.expandNearest、app.activity.collapseNearest、app.session.popup、app.session.sidebar、app.session.detailsCycle、app.session.sidebarWidth。空默认和弦时这些动作 不得响应任何按键；用户经 keybindings.json 重绑后 必须恢复响应。绑定目录 不得因默认和弦为空而移除这些 id。

    场景: 收缩后默认和弦不触发动作
      假如 产品绑定目录使用默认配置
      当 依次按下 Ctrl+T 与 Ctrl+O 与 Alt+E 与 Alt+Shift+E 与 Alt+I 与 Alt+U 与 Alt+O 与 Alt+Shift+U
      那么 思维折叠默认态未变且工具块展开态未变且详情与侧栏未开关

    场景: 用户重绑后动作恢复响应
      假如 用户在 keybindings.json 将 app.session.sidebar 绑到 ctrl+b
      当 在编辑器槽按下 Ctrl+B
      那么 会话侧栏按既有开启路径上屏
  @req:r3033
  规则: dock-action-chips
    产品 TUI 必须在 编辑区上边框右端绘制视图动作 chip：[详情] 与 [侧栏]，两者 必须可左键点击并分别走与动作 id 相同的开启或关闭路径；对应视图开启时 chip 必须反色标识激活态。标准档（≥80 列）必须同时绘制两者；窄于标准档 必须只绘制 [详情]，点 [侧栏] 不得出现（无 chip 即无入口）。存在非默认折叠态或逐块覆盖时 必须在标准档及以上额外绘制 [重置折叠]，点击等价旧 app.tools.blocks 动作：翻转工具族默认展开态并清空逐块覆盖。chip 与编辑区上边框同高：宽度不足时 必须整 chip 不绘制，不得拆半或换行；不得因此增高编辑区或另加固定区行。

    场景: 编辑区上边框右端视图 chip 可点开侧栏
      假如 产品 TUI 就绪于 120 列终端
      当 左键单击 编辑区上边框右端 [侧栏] chip
      那么 右侧栏上屏且内容区让宽且 [侧栏] chip 反色

    场景: 窄终端只有详情 chip
      假如 产品 TUI 就绪于 70 列终端
      当 渲染编辑区上边框
      那么 编辑区上边框右端只有 [详情] chip 且无 [侧栏] chip
  @req:r3034
  规则: dock-click-absolute-row
    固定区内的可点击元素 必须按终端绝对行命中，不得受 transcript 行数上限与视口滚动位置影响；同一屏幕格在滚动前后 必须命中同一动作。队列条 必须整条可点并走与 app.message.dequeue 相同的取回路径。未命中任何固定区元素的点击 不得被应用层吞掉。

    场景: 队列条点击取回排队文本
      假如 产品 TUI 忙碌且已排队一条 follow-up
      当 左键单击队列条
      那么 队列文本回到编辑器且队列清空
  @req:r3035
  规则: slot-row-click
    替换 editor 槽的列表面（会话树、resume、models、themes、MCP、场景）必须支持行点击：单击未选中行 必须只改变选中项且不执行动作；在已是当前选中行的同一行再点击 必须执行该行的主动作（与 Enter 同后果）。不采用双击计时。进行中或不可执行的槽态（resume 加载中）行点击 必须无操作。

    场景: 槽行首次点击只选中再次点击执行
      假如 打开会话树 槽且当前选中第一行
      当 左键单击第三行
      那么 选中项变为第三行且未发起 travel
      当 再左键单击第三行
      那么 发起一次 travel 且目标为第三行节点
  @req:r3036
  规则: slot-close-and-decision-chips
    每个替换 editor 槽的列表面 必须在其头行右端绘制可点击 [关闭] chip，点击走既有关槽路径；import 确认 必须绘制 [是] 与 [否] chip 并走既有决策路径；ask 问卷 必须绘制 [提交] 与 [跳过] chip 且选项行可点选，点击走既有选择与提交路径；capabilities 盘 行点击 必须直接切换该项开关（可逆，不经二次确认）。

    场景: 槽关闭 chip 回到编辑器
      假如 以斜杠命令打开模型选择器槽
      当 左键单击槽头 [关闭] chip
      那么 槽关闭且编辑器槽恢复且未提交模型变更

    场景: 能力盘行点击直接切换
      假如 打开能力覆盖盘
      当 左键单击某能力行
      那么 该能力开关翻转一次
  @req:r3037
  规则: echo-without-key-names
    产品 scrollback 与固定区的可点击提示 不得写组合键名称，必须以动作语义表达可点性：可展开块头 不得出现 () 包裹的 Ctrl+T / Alt+E 和弦；输出视口提示 必须为 click to expand / click to fold；队列条提示 必须为 click 语义（不得为 Alt+Up）；活动信封与簇头 不得拼接和弦旁注。折叠三角仍必须可见。中断回显 Running… 的 Esc 提示 必须保留。

    场景: 块头不写组合键
      假如 以场景构建器回放含思维块与工具块的序列
      当 渲染 scrollback
      那么 思维块与工具块头均不含括号组合键且含折叠三角
