# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-paste-burst

  @req:pb01
  场景: burst-detected-after-8-fast-chars
    假如 8 个 plain chars 各 1ms 间隔经 on_plain_char 到达
    当 随后立即调用 should_insert_newline_instead_of_submit
    那么 返回 true（Enter 应插入 newline）

  @req:pb01
  场景: no-burst-slow-typing
    假如 8 个 plain chars 各 20ms 间隔（慢于 8ms threshold）
    当 调用 should_insert_newline_instead_of_submit
    那么 返回 false（正常打字非 paste burst）

  @req:pb01
  场景: fewer-than-8-chars-no-burst
    假如 仅 5 个 fast chars 到达
    当 调用 should_insert_newline_instead_of_submit
    那么 返回 false（低于 8-char 最小值）

  @req:pb02
  场景: time-boundary-7ms-inside
    假如 两个 chars 间隔 7ms 到达
    当 检查 inter-char gap 对 8ms threshold
    那么 7ms 在 threshold 内，consecutive counter 递增

  @req:pb02
  场景: time-boundary-9ms-outside
    假如 两个 chars 间隔 9ms 到达
    当 检查 inter-char gap
    那么 9ms 超出 threshold，consecutive counter 重置为 1

  @req:pb03
  场景: enter-submits-after-suppress-window
    假如 检测到 burst 后 121ms 无新 chars
    当 在 +121ms 调用 should_insert_newline_instead_of_submit
    那么 返回 false（120ms suppress 窗口已过期）

  @req:pb03
  场景: reset-clears-state
    假如 burst 进行中且 reset() 被调用
    当 之后调用 on_plain_char 与 should_insert_newline
    那么 检测器行为如同无字符到达（无 burst）
