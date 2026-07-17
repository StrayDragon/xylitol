# language: zh-CN
# managed by llman sdd partition-migrate
功能: test-qa-gate

  @req:qg01
  场景: qa-runs-full-gate
    假如 开发者准备提交
    当 运行 just qa
    那么 fmt、clippy、workspace 测试、xylitol-tui 包测、doc、token check、scripts/check_*、prek 均执行且全部通过才打印成功

  @req:qg01
  场景: qa-runs-scripts-checks
    假如 仓库含 scripts/check_*.py
    当 运行 just qa
    那么 check-scripts-wired 与 check-scripts 被执行且全部通过才打印成功

  @req:qg02
  场景: qa-skips-e2e-by-default
    假如 本机未安装 tmux
    当 仅运行 just qa
    那么 第 5 层 E2E 不被调用；闸门可绿

  @req:qg03
  场景: docs-point-to-recipes
    假如 读者打开根 AGENTS.md 或 test-tui-harness skill
    当 查找验证入口
    那么 文档写明 just qa 为日常满闸、just qa-e2e 含真终端 E2E

  @req:qg04
  场景: wired-meta
    假如 新增 scripts/check_foo.py
    当 运行 just check-scripts-wired
    那么 因 check-scripts 通配或显式引用而通过；未入闸则失败

  @req:qg05
  场景: pty-opens-tree-slot
    假如 产品 Fake PTY 已就绪且已有至少一轮对话
    当 双 Esc 开树
    那么 屏幕含 Type to search 或 Search: 且含 fold/unfold 或 filters
