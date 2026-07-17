# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-git

  @req:g1
  场景: discovery
    假如 cwd 在 git repo 或 worktree 内
    当 find_git_repo 被调用
    那么 正确返回 repo path、common git dir 与 HEAD path

  @req:g2
  场景: branch
    假如 HEAD 指向 refs/heads/main 或为 detached
    当 get_current_branch 被调用
    那么 命名 ref 返回 main，detached 返回 null

  @req:g3
  场景: watch
    假如 HEAD 文件上设置了 file watcher
    当 HEAD 文件被修改
    那么 分支变更回调被调用

  @req:g4
  场景: urls
    假如 提供 SCP、HTTPS、SSH 与 git 格式 URL
    当 parse_git_url 被调用
    那么 每种格式正确提取 host 与 path
