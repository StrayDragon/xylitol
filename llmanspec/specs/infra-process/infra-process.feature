# language: zh-CN
# capability: infra-process
# purpose: Shell 进程管理 — 子进程 spawn、进程组与取消。
# scope: src/infra/process/, src/protocol/

功能: infra-process

  @req:r1493
  规则: bash-discovery
    System MUST 跨平台定位可用的 bash：Windows 上 Git Bash，Unix 上 /bin/bash，回退到 sh。
    # verified-by: src/infra/bash_exec/mod.rs
  @req:r1494
  规则: shell-env
    System MUST 构造注入 agent bin 目录后的 shell PATH 环境。
    # verified-by: src/infra/bash_exec/mod.rs
  @req:r1495
  规则: process-group
    System MUST 支持按进程整树终止：在所有平台上结束目标进程及其所有子进程。
    # verified-by: src/infra/bash_exec/mod.rs
  @req:r1496
  规则: child-wait
    System MUST 等待子进程退出并取得 exit code，MUST NOT 因 detached 后代进程持有继承 stdio handle 而挂起。
    # verified-by: src/infra/bash_exec/mod.rs
# re-review(c2835): 复审结论——本 capability 管辖行为不变；仅协议载体常量与死变体清理。（2026-09-29）
