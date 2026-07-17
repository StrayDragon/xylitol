# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-process

  @req:p1
  场景: macos
    假如 macOS 且存在 /bin/bash
    当 find_bash 被调用
    那么 bash path 为 /bin/bash 且 shell config 使用 -c

  @req:p2
  场景: path
    假如 agent bin dir 为 /home/user/.xylitol/bin
    当 build_shell_env 被调用
    那么 结果 PATH 含 /home/user/.xylitol/bin

  @req:p3
  场景: kill-tree
    假如 进程及其子进程运行中
    当 kill_process_tree(pid) 被调用
    那么 进程及所有子进程收到 SIGKILL

  @req:p4
  场景: wait
    假如 子进程已退出但 stdout pipe 仍打开
    当 wait_for_child 被调用
    那么 函数等待 pipe idle 后返回 exit code
