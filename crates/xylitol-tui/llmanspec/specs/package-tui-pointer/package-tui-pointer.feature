# language: zh-CN
# capability: package-tui-pointer
# purpose: ApplicationOwned 指针命中管线：正文行 / 可见格命中区、click 相位、探针与 capture。

功能: package-tui-pointer
  背景:
    假如 ApplicationOwned 应用会话已 begin 且 transcript 有可拖选文本

  @req:r1931
  规则: pointer-hit-regions
    ApplicationOwned 须 提供以不透明数值 id 标识的命中区。钉在正文行的命中 须 用 content 坐标（行、列区间、可选多行高度），屏幕格到 id 的映射 须 使用事件时刻的应用视口滚动位置，不得 使用登记时冻结的 scroll 快照。钉在可见页的命中 须 按屏幕格匹配，滚动后同一屏幕格仍命中同一 id。空表 须 等同未登记命中。不得 把产品折叠种类编进库类型。

    场景: 命中标识跟随在线滚动
      当 登记 content 命中区并改变视口滚动后查询同一屏幕格
      那么 命中 id 随 live 滚动变化而不是登记时的快照

    场景: 视口命中保持时屏幕单元格
      当 登记视口命中区并改变视口滚动后查询同一屏幕格
      那么 命中 id 仍钉在同一屏幕格
  @req:r1932
  规则: pointer-click-phase
    ApplicationOwned 指针 click 相位默认 须 为按下即吞（Left Down 命中 id 则 不得 启动 transcript 拖选；随后 Drag 与 Up 不得 再报告 click）。库 须 提供 opt-in：仅当选区为空且指针未拖动时在 Left Up 报告 click；该相位下空 click 不得 发出复制。拖动过程 不得 报告 click。

    场景: 缺省向下吞掉向上静默
      假如 click 相位为 "DownSwallow"
      当 在命中区 Left Down 后未拖动即 Left Up
      那么 按下吞掉拖选且松开不报告 click

    场景: 向上若空点击无复制
      假如 click 相位为 "UpIfEmptySelection"
      当 在命中区 Left Down 后未拖动即 Left Up
      那么 报告 click 且不发出 OSC52
  @req:r1933
  规则: pointer-hover-probe
    ApplicationOwned 须 允许武装同一鼠标输入管道上的指针探针（不得 另建第二套鼠标事件源）。未武装时无态 Moved 不得 强制整帧重绘。武装后 Moved 须 送达探针；同一命中 id 连续 Moved 不得 排帧；id 变化 MAY 请求重绘。

    场景: 未武装移动无完整帧
      当 未武装探针时派发 Moved
      那么 不强制整帧重绘

    场景: 已武装悬停跳过未变标识
      假如 指针探针已武装
      当 连续两次 Moved 落在同一命中 id
      那么 第二次不排帧
  @req:r1934
  规则: pointer-capture-before-selection
    ApplicationOwned 在启动 transcript 拖选之前 须 先判定 capturing 命中区（含盖在 transcript 上的 overlay 几何）。Left Down 命中 capturing 区后直到松开，Drag 不得 启动 transcript 选区。

    场景: 捕获块誊本拖选
      假如 已登记 capturing 命中区
      当 在 capturing 区 Left Down 并拖动
      那么 不启动 transcript 拖选
  @req:r1935
  规则: pointer-cheap-hover-paint
    由指针探针引起的悬停呈现 须 能在不重建全部组件的前提下绘出（可见行 overlay 或等价 reproject）。不得 因此将组件树标为整段 stale。悬停绘出 须 独立于 transcript 选区反色。

    场景: 悬停绘制会不过期组件
      假如 指针探针已武装
      当 Moved 导致命中 id 变化
      那么 请求重绘且组件树未被标为整段 stale
  @req:r2894
  规则: pointer-dock-absolute-row
    指针命中区 必须提供固定区（dock）空间：该空间的命中行 为终端绝对行，不参与 transcript 行数上限判断，也不随视口滚动变化。同一屏幕格在滚动前后 必须命中同一动作。transcript 空间（content / viewport）的行门与滚动语义 不得因此改变。悬停呈现 只处理 transcript 行；固定区行 不得因命中区登记而被悬停着画。

    场景: 固定区命中不受滚动与行数上限限制
      假如 登记一个终端绝对行的固定区命中区
      当 视口滚动后按该绝对行查询
      那么 仍命中同一 id 且该行超出 transcript 行数也不影响

    场景: 悬停不着画固定区行
      假如 固定区命中区已登记且探针已武装
      当 悬停落在固定区命中区上
      那么 悬停呈现不修改任何 transcript 行
