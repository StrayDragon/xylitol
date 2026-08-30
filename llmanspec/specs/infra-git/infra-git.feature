# language: zh-CN
# capability: infra-git
# purpose: Git 仓库操作 — 分支管理、仓库发现与 URL 解析。
# scope: src/infra/git/

功能: infra-git

  @req:g1 @human
  场景: repo-detection
    - System MUST 定位所在 git 仓库：向上遍历目录查找 .git 目录或 worktree 文件，返回 repo path、common git dir 与 HEAD path。

  @req:g2 @human
  场景: branch-detection
    - System MUST 提供 get_current_branch(git_dir)，读取 HEAD 引用并返回当前分支名，detached HEAD 时返回 null。

  @req:g3 @human
  场景: branch-watch
    - System MUST 支持经文件系统 watcher 监视 HEAD 文件的分支变更。

  @req:g4 @human
  场景: url-parsing
    - System MUST 提供 parse_git_url(url)，处理 SCP-like、HTTPS、SSH 与 git protocol URL，并提取 ref。
