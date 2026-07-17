# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-clipboard

  @req:r13
  场景: macos
    假如 macOS 系统且 pbcopy 可用
    当 copy_to_clipboard 被调用并传入 text
    那么 pbcopy 进程从 stdin 接收 text 且剪贴板含 text

  @req:r13
  场景: linux-wayland
    假如 Wayland Linux 且 wl-copy 可用
    当 copy_to_clipboard 被调用并传入 text
    那么 wl-copy 经 stdin pipe 接收 text 且剪贴板含 text

  @req:r13
  场景: linux-x11
    假如 X11 Linux 且 xclip 可用
    当 copy_to_clipboard 被调用并传入 text
    那么 xclip 接收 text 且剪贴板含 text

  @req:r14
  场景: fallback-chain
    假如 所有原生剪贴板工具均失败
    当 copy_to_clipboard 被调用并传入小文本
    那么 stdout 发出 OSC 52 转义序列

  @req:r15
  场景: image-read
    假如 wl-paste 或 xclip 报告图片 MIME 类型
    当 read_clipboard_image 被调用
    那么 返回含 bytes 与 mime_type 的 ClipboardImage

  @req:r16
  场景: ssh-session
    假如 SSH_CONNECTION 已设置且原生剪贴板工具不可用
    当 copy_to_clipboard 被调用并传入小文本
    那么 stdout 发出 OSC 52 序列

  @req:r17
  场景: oversize
    假如 text 超过 100KB base64 编码
    当 copy_to_clipboard 被调用并传入大文本
    那么 跳过 OSC 52 回退并返回错误

  @req:r18
  场景: all-fail
    假如 系统无剪贴板工具且无终端
    当 copy_to_clipboard 被调用
    那么 返回描述性错误字符串

  @req:r19
  场景: write-png-temp
    假如 提供 image/png 字节
    当 写出 tempfile
    那么 路径存在且后缀为 .png

  @req:r20
  场景: wayland-plain-text
    假如 Wayland 且 wl-paste 报告 text/plain
    当 调用 read_clipboard_text
    那么 返回非空 UTF-8 字符串

  @req:r20
  场景: empty-clipboard
    假如 剪贴板无 text/plain
    当 调用 read_clipboard_text
    那么 返回 None 或空串语义且不 panic
