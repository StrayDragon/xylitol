# c140-add-git-utils: Tasks

## Implementation

- [ ] 创建 `src/infra/git/mod.rs` — 模块入口
- [ ] 创建 `src/infra/git/repo.rs` — 仓库检测（.git 遍历 + worktree 支持）
- [ ] 创建 `src/infra/git/branch.rs` — 当前分支检测
- [ ] 创建 `src/infra/git/url.rs` — Git URL 解析
- [ ] 添加 feature flag `infra-git` 到 `Cargo.toml`
- [ ] 添加 `git2` 和 `notify` 依赖

## Testing

- [ ] 单元测试 — 仓库检测（标准目录 + worktree）
- [ ] 单元测试 — 分支检测（命名分支 + detached HEAD）
- [ ] 单元测试 — URL 解析（SCP、HTTPS、SSH、git 协议）
- [ ] 单元测试 — 分支监控回调

## Verification

- [ ] `cargo check --features infra-git`
- [ ] `cargo test --lib --features infra-git`
- [ ] `llman sdd validate c140-add-git-utils`
