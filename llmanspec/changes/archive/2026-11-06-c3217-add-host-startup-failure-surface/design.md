# Design — Surface host startup failure fast

## 1. 子进程句柄

- SpawnedHost { child: Option<Child>, stderr_path: Option<PathBuf> }。
- spawn_detached_serve 返回 SpawnedHost；stderr 指向 0600 临时文件。
- Drop 时把仍在运行的 child 交给等待线程（detach），并删除临时文件。

## 2. 就绪等待

- wait_until_ready 每轮 try_wait；Some(status) 即返回 host_exited_message。
- 消息含：URL、退出状态、stderr 末尾 4 KiB、日志路径。
- 成功就绪时 Drop detach，不影响 Host 继续运行。

## 3. TUI 提示

- CLI ensure-host 分支先用 attach_preflight 探测；未在听时输出 starting local Host at <url> ...。
- 该提示走 stderr，TUI 进入备用屏前可见。

## 4. 边界

- 不改变 30s ready timeout；不自动重试。
- 不解析 Host 日志内容，只展示 stderr 摘要。
- Windows 下 stderr 临时文件删除失败可忽略。

## 5. 测试

- 单测：spawn 一个立即退出的 sh，stderr 写错误文本；ensure_host_with 快速返回含 exited before becoming ready 与 stderr 摘要。
- 现有 ensure_host_* 测试的 spawn 闭包改为返回 SpawnedHost::detached()。
- 手测：无效配置下 TUI 打印 starting 提示，并在 Host 子进程退出后立即返回错误；本次实测 10s（Host 自身启动失败后退出耗时），未等到 30s ready timeout。
