# language: zh-CN
功能: Bash工具
  作为一个 LLM 代理
  我想要执行 shell 命令
  以便运行开发工具和检查系统状态

  背景:
    假定 有一个临时工作目录

  场景: 执行简单命令
    当 调用bash命令 "echo hello world"
    那么 退出码为 0
    并且 stdout 包含 "hello world"

  场景: 捕获 stderr 输出
    当 调用bash命令 "echo 错误信息 >&2"
    那么 stdout 和 stderr 合并输出包含 "错误信息"

  场景: 报告非零退出码
    当 调用bash命令 "exit 42"
    那么 退出码为 42

  场景: 命令超时被强制执行
    当 调用bash命令 "sleep 10" 超时 1 秒
    那么 命令应该失败 包含超时错误

  场景: stdout 和 stderr 合并输出
    当 调用bash命令 "echo out && echo err >&2 && echo out2"
    那么 stdout 和 stderr 合并输出包含 "out"
    并且 stdout 和 stderr 合并输出包含 "err"
    并且 stdout 和 stderr 合并输出包含 "out2"

  场景: 输出超过限制时截断
    当 调用bash命令 "yes '长文本行' | head -10000"
    那么 输出被截断
    并且 截断详情显示达到字节或行限制

  场景: 取消信号杀掉进程树
    当 调用bash命令 "sleep 60"
    并且 在500ms后发送取消信号
    那么 命令应该失败 包含取消错误

  场景: 缺少命令参数被拒绝
    当 调用bash 不传命令参数
    那么 调用失败 包含验证错误
