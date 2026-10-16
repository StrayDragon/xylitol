<!-- LLMANSPEC:START -->

# llmanspec AGENTS.md

This file is referenced by the root `AGENTS.md` managed block. Use it to add
project-specific rules, context, or conventions that you want AI agents to follow.

<!-- Add your rules below this line -->
<!-- LLMANSPEC:END -->

# 包实例例外（xylitol-tui）

- 行为合约 SSOT 在本目录 `specs/`。根 `llmanspec/changes/<id>` 绑定的是**根**实例；**不要**在本包再建 `changes/<id>/specs/`。
- 根 change 可标 `needs_specs_change: false`（根 `llmanspec/specs/` 无落地），本包 live `.feature` 仍须在绑定分支改并提交。
- `llman-sdd review --capability package-tui-*` / `validate --specs` 在包根跑（cwd 或 `--directory crates/xylitol-tui`）。根实例看不到这些 capability。
- feature `# scope:` 写**本包相对路径**（如 `src/components/markdown.rs`），不要写仓根 `src/`——宽 scope 会让无关 crate 文件把 `--strict` staleness 打红。
- 包实例 `validate --strict` 的 STALE 可能是假阳性：llman-sdd 0.7.1 会 `liftScope` 给代码路径加 `crates/xylitol-tui/` 前缀，但 `specRel` 仍是实例相对 `llmanspec/specs/…`，对不上 `git diff --name-only` 的仓根路径。feature 文件其实已改，`--strict` 仍报「spec was not updated」。归档门禁是根实例 `validate <change-id> --strict` 与无旗标 `review`；包实例用 `validate --check`（不要 `--strict`）即可。根因在工具，不要靠反复改 feature 消这条 STALE。
