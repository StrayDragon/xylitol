# language: zh-CN
# purpose: 剪贴板集成 — 原生剪贴板、OSC 52 与图片复制支持。

功能: infra-clipboard

  @req:r13
  规则: text-copy
    System 须提供 copy_to_clipboard(text)，经平台原生工具将 UTF-8 文本复制到系统剪贴板。
    场景: 拖选选择复制助手正文
      假如 ApplicationOwned 应用会话已 begin 且 transcript 有可拖选文本
      当 未修饰左键拖选非空范围并松开
      那么 发出 OSC52 剪贴板序列

  @req:r14
  规则: platform-order
    System 须按顺序尝试平台原生剪贴板：native addon > pbcopy/clip > wl-copy > xclip/xsel > termux-clipboard-set > OSC 52。
    场景: 缺省模式行内
      假如 新建默认 TUI
      当 查询交互模式
      那么 模式为 Inline 且应用会话未激活

  @req:r15
  规则: image-clipboard
    System 须在支持平台上读取剪贴板图片数据并检测 MIME 类型。
    场景: 复制提示可观察
      假如 ApplicationOwned 应用会话已 begin 且 transcript 有可拖选文本
      当 松手复制成功
      那么 copy-notice 信号可观察且空选不发

  @req:r16
  规则: osc52-remote
    原生剪贴板不可用时，System 须对远程会话（SSH）回退到 OSC 52 转义序列。
    场景: OSC52远端回退
      假如 ApplicationOwned 应用会话已 begin 且 transcript 有可拖选文本
      当 未修饰左键拖选非空范围并松开
      那么 发出 OSC52 剪贴板序列

  @req:r17
  规则: osc52-limit
    System 须遵守 OSC 52 载荷大小限制（编码后 100KB），超出时 须跳过 OSC 52 回退。
    场景: 空选区静默
      假如 ApplicationOwned 应用会话已 begin 且 transcript 有可拖选文本
      当 松手复制成功
      那么 copy-notice 信号可观察且空选不发

  @req:r18
  规则: clipboard-error
    所有剪贴板方法均失败时，System 须返回描述性错误，不得静默吞掉失败。
    场景: 编辑器复制路径仅
      假如 ApplicationOwned 下 Editor 有多行缓冲
      当 在 Editor 内未修饰拖选跨行并松开
      那么 仅输入缓冲文本进入复制路径且 transcript 选区未写入

  @req:r19
  规则: clipboard-image-temp-file
    System 须提供将剪贴板图片字节写入 tempfile 的能力：文件名含不可预测分量；扩展名由 MIME 推导；返回绝对路径。不得写入固定世界可读路径（如 ~/.xylitol/paste.png）。写盘失败 须返回描述性错误。
    场景: 图片粘贴来自Host
      当 以主机泵注入剪贴板图片后按粘贴键
      那么 编辑器插入落盘路径文本且路径经 Driver 暂存

  @req:r20
  规则: clipboard-text-read
    System 须提供 read_clipboard_text：在支持平台上读取系统剪贴板 UTF-8 文本；无文本时返回空/None；工具不可用或失败 须返回描述性错误。不得依赖 OSC 52 读取（OSC 52 仅用于写出回退）。
    场景: 文本读取经Host泵
      当 以主机泵完成一轮含已提交 assistant 的对话后提交 "/history-copy-last"
      那么 剪贴板收到 assistant 正文且未复制 thinking
