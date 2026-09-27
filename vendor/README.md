# Vendored patches

本目录是对第三方 crate 的**本地补丁副本**，经根 `Cargo.toml` 的
`[patch.crates-io]` 接入。不向上游提 PR；重 vendoring 时按下列 diff 重放。

## gherkin（基于 crates.io 0.16.0，版本号保持 0.16.0）

rstest-bdd 0.6 stable（runtime 与 macros）均以 `gherkin = "0.16"` 依赖此
crate，且其场景选择只遍历顶层 `Feature::scenarios`。而 llman-sdd 0.5 的
规范格式是原生分层 Gherkin（`规则:` 块 + 嵌套 `场景:`）。两处补丁：

1. `src/languages.json`：zh-CN `rule` 补官方同义词 `规则`
   （cucumber 官方 gherkin-languages.json 为 `["Rule", "规则"]`；
   crate 0.14 与 0.16 的内嵌快照均只有 `["Rule"]`）。`build.rs` 会在编译期
   从该 JSON 重新生成关键字表，无需其他改动。
2. `src/parser.rs` `feature()` 动作：把 `rules[].scenarios` 按文档序
   （`position.line, position.col`）提升进 `Feature::scenarios`，
   `rules` 仍保留在 AST 中。已知局限：rule 级 `背景:` 不会被提升场景继承
   （本仓库 specs 无 rule 级背景）。

重 vendoring 流程：

```bash
rm -rf vendor/gherkin
cp -r ~/.cargo/registry/src/*/gherkin-<ver> vendor/gherkin
rm -f vendor/gherkin/.cargo_vcs_info.json vendor/gherkin/.cargo-ok
# 重放上述两处 diff；核对 crate 版本号与依赖方要求一致
```

注意：`vendor/` 不在 workspace 成员内，`just fmt` / clippy 不会触碰它；
保持上游原始排版，diff 只留补丁本身。
