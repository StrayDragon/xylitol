# Design: Resource Diagnostics（只读清单与诊断）

## 为什么大幅缩范围（决策记录）

1. **生态不可复用**：pi 的 npm 包主体是 TS 扩展（jiti 加载 JS），Rust 无法执行；独立发布的纯文本 skill/prompt 包几乎没有。npm registry/semver/tarball 这套对 Rust 侧无价值。
2. **自用优先**：用户主要自用，手动 `git clone` 到 `~/.xylitol/skills/` 或项目 `.xylitol/skills/` 已足够，ResourceLoader（c60 已归档）已能发现。
3. **砍 install 后 update/remove 无基础**：来源元数据（git url/版本/启用状态）由 install 建立；用户手动放置的资源系统不知来源，update/remove 无可靠对象，强做只能做"扫目录猜来源"的脆弱逻辑。
4. **只读自洽**：list/info/doctor 只需复用 ResourceLoader 已有的发现 + 诊断结果，零状态、自洽、价值确定。

## 决策

1. **复用而非重扫**：所有命令走 `DefaultResourceLoader` 已加载的结果，不独立 rescan，保证 CLI 所见与运行时（agent 实际加载的资源）完全一致。
2. **来源范围**：用 ResourceLoader 已区分的 project/user scope 展示（c60 已实现 project 先于 user、同名 project 胜出），不重新发明 scope 概念。
3. **doctor 退出码**：无诊断问题 exit 0；有问题 exit 非零，便于脚本/CI 集成与未来的 `--check` 钩子。
4. **CLI 形态**：`xylitol resources list|info <name>|doctor`。与未来 RPC（c115）的 `get_resources` 共享同一 ResourceLoader 数据源，避免双份扫描逻辑。
5. **只读保证**：命令路径只调用 `ResourceLoader` 的 getter 与 `diagnostics()`，不触任何写 API；设计上杜绝副作用。
6. **无新依赖**：纯 std + clap（已有）。

## 不做

- 不做 install/update/remove（手动 git 操作即可）。
- 不做 npm/git/local 多源安装。
- 不做来源元数据登记（无 install 自然不需要）。
- 不做 themes 渲染 / extensions（无 TUI / 无扩展加载器）。
