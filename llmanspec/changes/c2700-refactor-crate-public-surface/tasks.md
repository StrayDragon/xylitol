# Tasks: c2700-refactor-crate-public-surface

> pre-start：全部未勾选。不 `change start`、不改 live specs、不改代码，直到批次 review 认领。
> `skip_specs_landing: true`。实现层重构；若发现 spec 钉了模块路径，在绑定分支**直接编辑** live spec 去路径，不新增产品 MUST。

## 1. 认领闸

- [ ] 1.1 确认本机在认领分支（review 通过后才 `change start`）；工作区无其它 active change 缠斗。
- [ ] 1.2 列出 `rg 'xylitol::(agent|infra|app)::' tests packages src` 的外部风格 import，作为迁移清单。

## 2. 可见性

- [ ] 2.1 [blocked-by: 1.2] `src/lib.rs`：`agent`/`infra` → `pub(crate)`；决定 `app`/`utils`/`protocol` 子模是否同样收。按 `design.md` §3。
- [ ] 2.2 [blocked-by: 2.1] 修主 crate 内编译：测试改 `crate::`；`embed` 只 re-export 仍公开的类型。
- [ ] 2.3 [blocked-by: 2.2] BDD / integration 凡 `xylitol::infra` / `xylitol::agent::capabilities` 改为 Driver / embed / 同 crate。
- [ ] 2.4 [blocked-by: 2.3] rustdoc：`lib.rs` + `embed.rs` 与「已知泄漏」对齐；禁止再写「可 reach infra」。

## 3. 验证

- [ ] 3.1 [blocked-by: 2.4] `compile_fail` 或独立示例证明 `xylitol::infra` 不可用。
- [ ] 3.2 [blocked-by: 3.1] `just fmt` / 相关 clippy / `just qa`（认领方跑）。
- [ ] 3.3 [blocked-by: 3.2] `llman sdd validate c2700-refactor-crate-public-surface --strict --no-check`。

## 4. 交接

- [ ] 4.1 [blocked-by: 3.3] 在 proposal Further Notes 补一行实际 diff 范围（文件数级），供 `c2705` 接手。
