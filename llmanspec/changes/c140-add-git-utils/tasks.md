# c140-add-git-utils: Tasks

## Implementation

- [x] 创建 `src/infra/git/mod.rs` — 模块入口
- [x] 创建 `src/infra/git/repo.rs` — 仓库检测（.git 遍历 + worktree 支持）
- [x] 创建 `src/infra/git/branch.rs` — 当前分支检测（命名分支 + detached HEAD）
- [x] 创建 `src/infra/git/url.rs` — Git URL 解析（SCP、HTTPS、SSH、git 协议）
- [x] 添加 feature flag `infra-git` 到 `Cargo.toml`
- [x] 添加 `url` 依赖

## Testing

- [x] 单元测试 — 分支检测（命名分支 + detached HEAD）
- [x] 单元测试 — URL 解析（SCP、HTTPS、SSH、带 ref）
- [x] 单元测试 — 无效 URL 返回 None

## Verification

- [x] `cargo check --features infra-git` — 0 errors
- [x] `cargo test --lib --features infra-git` — 7 passed
- [x] `llman sdd validate c140-add-git-utils`
