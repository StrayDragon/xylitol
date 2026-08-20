# language: zh-CN
# Migrated from llmanspec/specs/cli-entry/cli-entry.feature (hand-written BDD chain).
# Bound in tests/bdd.rs; remaining cli-entry constraints are feature:false in spec.toon.
功能: cli-entry

  @req:ce15
  场景: tokenizer-help-tree
    假如 CLI 已解析
    当 xylitol tokenizer --help
    那么 可见 status、download、clean 叶子

  @req:ce15
  场景: tokenizer-status-empty
    假如 缓存目录为空
    当 xylitol tokenizer status
    那么 报告缓存根且不失败伪装已下载

  @req:ce15
  场景: tokenizer-download-opt-in
    假如 目标映射到 HuggingFace 且本地无缓存
    当 xylitol tokenizer download <target> --yes
    那么 词表落入缓存路径且再次 status 可见

  @req:ce15
  场景: tokenizer-clean
    假如 缓存中已有条目
    当 xylitol tokenizer clean --all
    那么 条目被移除且 status 不再列出

  @req:ce15
  场景: tokenizer-no-bootstrap
    假如 仅执行 tokenizer 子命令
    当 xylitol tokenizer status
    那么 不经 bootstrap 装配会话或 MCP 即可完成

  @req:ce15
  场景: tokenizer-download-shows-hf-base
    假如 已设置 HF_ENDPOINT 为镜像基址且目标已映射
    当 xylitol tokenizer download <target> 进入确认摘要（或 --yes 的等价日志）
    那么 摘要含该镜像基址与落盘路径

  @req:ce16
  场景: surface-tui-verb
    假如 TTY
    当 xylitol tui
    那么 进入产品 TUI

  @req:ce16
  场景: surface-print-verb
    假如 无 prompt
    当 xylitol print
    那么 错误退出且无 Hello!

  @req:ce16
  场景: ops-stay-toplevel
    假如 CLI 已解析
    当 xylitol --help
    那么 Commands 含 tokenizer、resources 与 serve 为顶层而非 tui 子命令

  @req:ce8
  场景: serve-ops-verb
    假如 CLI 已解析
    当 xylitol serve --help
    那么 可见 --host --port 与 stop、install，且无 server 或 run 别名

  @req:ce19
  场景: surface-flags-on-tui
    假如 表面旗标上下文就绪
    当 xylitol tui --session sid --model m --trust
    那么 解析成功且表面旗标生效

  @req:ce19
  场景: surface-flags-on-tui-run
    假如 表面旗标上下文就绪
    当 xylitol tui run --session sid
    那么 解析成功且 --session 生效

  @req:ce19
  场景: surface-flags-on-print
    假如 表面旗标上下文就绪
    当 xylitol print --session sid --no-color hi
    那么 解析成功

  @req:ce19
  场景: surface-flags-on-print-trust
    假如 表面旗标上下文就绪
    当 xylitol print --session sid --trust --model m hi
    那么 解析成功且表面旗标生效

  @req:ce19
  场景: surface-flags-on-print-no-trust
    假如 表面旗标上下文就绪
    当 xylitol print --session sid --no-trust hi
    那么 解析成功且表面旗标生效

  @req:ce19
  场景: toplevel-surface-flags-rejected
    假如 表面旗标上下文就绪
    当 xylitol --session sid
    那么 解析失败

  @req:ce20
  场景: resume-hint-when-persisted
    假如 当前 session 已出现在 list_sessions
    当 TUI 或 print 正常退出
    那么 stderr 含 resume 提示行

  @req:ce20
  场景: resume-hint-absent-when-unpersisted
    假如 当前 session 未持久化
    当 TUI 或 print 正常退出
    那么 stderr 不含 resume 提示行

  @req:ce21
  场景: tui-attach-host-down
    假如 本机 Host 未在听
    当 产品 TUI 尝试 attach
    那么 非零退出并提示用 xylitol serve 启动 Host
