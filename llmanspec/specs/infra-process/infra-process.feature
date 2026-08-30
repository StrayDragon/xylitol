# language: zh-CN
# capability: infra-process
# purpose: Shell 进程管理 — 子进程 spawn、进程组与取消。
# scope: src/infra/process/, src/protocol/

功能: infra-process

  @req:p1 @human
  场景: bash-discovery
    - System MUST 跨平台定位可用的 bash：Windows 上 Git Bash，Unix 上 /bin/bash，回退到 sh。

  @req:p2 @human
  场景: shell-env
    - System MUST 构造注入 agent bin 目录后的 shell PATH 环境。

  @req:p3 @human
  场景: process-group
    - System MUST 支持按进程整树终止：在所有平台上结束目标进程及其所有子进程。

  @req:p4 @human
  场景: child-wait
    - System MUST 等待子进程退出并取得 exit code，MUST NOT 因 detached 后代进程持有继承 stdio handle 而挂起。
