# language: zh-CN
# capability: package-tui-paste-burst
# purpose: packages/xylitol-tui 的 PasteBurst 检测器（非 bracketed paste 的 Enter 抑制）。
# scope: packages/xylitol-tui/, tests/

功能: package-tui-paste-burst

  @req:pb01 @human
  场景: paste-burst-detector
    - TUI MUST 提供 PasteBurst 检测器（纯状态机），识别 non-bracketed paste burst：8 个或以上 plain characters 彼此间隔 8ms 内到达。检测器 MUST 暴露 on_plain_char(now) 喂入字符到达，should_insert_newline_instead_of_submit(now) 查询即将到来的 Enter 是否应插入 newline 而非 submit。检测器 MUST NOT 缓冲字符（editor 正常插入文本）；仅决定 Enter 行为。镜像 pi paste-burst.ts。

  @req:pb02 @human
  场景: time-injected
    - PasteBurst 方法 MUST 接受 Instant 参数（now）而非内部读 wall-clock，使测试经 c405 MockClock 确定性推进时间而无需 thread::sleep。editor（未来 port）将在各 call site 传入 clock.now()。这实现 c405 spec tt04（timing-injectable-clock）的 paste-burst 情形。

  @req:pb03 @human
  场景: enter-suppress-window
    - 检测到 burst（8+ chars 在 inter-char threshold 内）时，PasteBurst MUST 打开 120ms enter-suppress 窗口，期间 should_insert_newline_instead_of_submit 返回 true，以及 30ms active-idle 窗口在字符间保持检测器 active。suppress 窗口在无新字符后过期，后续 Enter MUST 正常 submit。reset() MUST 清除所有状态（用于 non-printable keys、bracketed paste 或显式 disable）。
