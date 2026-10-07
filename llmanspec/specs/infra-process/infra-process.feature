# language: zh-CN
# capability: infra-process
# purpose: Shell 进程管理 — 子进程 spawn、进程组与取消。
# scope: src/infra/process/, src/protocol/

功能: infra-process

  @req:r1493
  规则: bash-discovery
    System MUST 跨平台定位可用的 bash：Windows 上 Git Bash，Unix 上 /bin/bash，回退到 sh。
    # verified-by: src/infra/bash_exec/mod.rs

    场景: bash-path-discovery
      当 请求跨平台 bash 定位
      那么 返回可执行 shell 配置
  @req:r1494
  规则: shell-env
    System MUST 构造注入 agent bin 目录后的 shell PATH 环境。
    # verified-by: src/infra/bash_exec/mod.rs

    场景: shell-env-path-lookup
      当 读取 shell 环境装配边界
      那么 以 PATH 定位可执行 bash
  @req:r1495
  规则: process-group
    System MUST 支持按进程整树终止：在所有平台上结束目标进程及其所有子进程。
    # verified-by: src/infra/bash_exec/mod.rs

    场景: kill-process-tree-reaps
      当 以整树终止子进程
      那么 目标进程及其子进程一并结束
  @req:r1496
  规则: child-wait
    System MUST 等待子进程退出并取得 exit code，MUST NOT 因 detached 后代进程持有继承 stdio handle 而挂起。
    # verified-by: src/infra/bash_exec/mod.rs

    场景: child-wait-with-reap-guard
      当 读取外部工具进程回收边界
      那么 等待退出取得状态且整树回收
# re-review(c2835): 复审结论——本 capability 管辖行为不变；仅协议载体常量与死变体清理。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2837): flaky-fix 分支复核——kill-tree 平台差异修复触及本 scope；行为不变。（2026-10-06）
