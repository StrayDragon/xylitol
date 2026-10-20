# language: zh-CN
# capability: package-tui-scrollbar-gutter
# purpose: ApplicationOwned transcript 最右 1 列滚动槽：跳转顶部/跳转底部按钮、宽度预留与拖选让位。

功能: package-tui-scrollbar-gutter

@req:r2891
规则: gutter-opt-in-viewport-stamp
  滚动槽必须为 ApplicationOwned 会话级 opt-in（默认关）。开启时 transcript 视口必须在最右保留 1 列：顶行绘跳转顶部按钮字符、底行绘跳转底部按钮字符、内域按滚动比例绘 thumb（比例域排除按钮行），thumb 之外绘轨道；内容行按预留后宽度排版且不得溢出终端宽。内容行数不超过视口高、或预留后可用宽低于极窄阈值时，不得绘制且不得预留。全渲染与滚轮重投影两路必须呈现一致。

  场景: 槽盖章时右边缘
    假如 ApplicationOwned 会话开启滚动槽且内容超过一屏
    当 渲染任一帧
    那么 transcript 最右一列为滚动槽且正文按少一列排版

  场景: 滑块反映滚动比率
    假如 ApplicationOwned 会话开启滚动槽且内容超过一屏
    当 向下滚动到中段后渲染
    那么 thumb 位置随滚动比例移动且按钮行不被 thumb 占用

  场景: 槽隐藏当无溢出
    假如 ApplicationOwned 会话开启滚动槽但内容不超过一屏
    当 渲染
    那么 不绘制滚动槽且正文占满全宽

@req:r2892
规则: gutter-capture-clicks
  滚动槽列必须以视口命中区暴露给宿主（非引擎 capturing 语义——capture 分支吞 pending_click，与 UpIfEmptySelection 相位不相容）：跳转顶部/跳转底部各区钉在当前可见页顶/底行最右列，滚动后仍钉在同一屏幕格。左键按下于滚动槽不得启动 transcript 拖选（拖选隔离由宿主选中 rect 按预留后宽 carve 保证）；点跳转顶部必须等价滚动到顶，点跳转底部必须等价滚动到底并重续跟随；轨道点击不得产生动作。拖选经过滚动槽不得产生选区。

  场景: 顶部点击滚动至头部
    假如 ApplicationOwned 会话开启滚动槽且已向下滚动离开顶部
    当 左键单击跳转顶部按钮字符
    那么 应用视口滚动位置归零

  场景: 底部点击再武装跟随
    假如 ApplicationOwned 会话开启滚动槽且已向上滚动离开底部
    当 左键单击跳转底部按钮字符
    那么 应用视口滚到底且跟随底部

  场景: 槽向下从不启动选区
    假如 ApplicationOwned 会话开启滚动槽
    当 在最右列按住左键拖动
    那么 不产生 transcript 选区

  场景: 轨道按下跳转按比例
    假如 ApplicationOwned 会话开启滚动槽且已向下滚动离开顶部
    当 在滑轨内域中段按下
    那么 滚动位置按比例即时改变且进入拖拽

  场景: 拖选跟随指针直到发布
    假如 滑轨拖拽进行中
    当 按住移动到另一内域行
    那么 滚动位置按抓取偏移持续跟随且抬起后不再跟随
