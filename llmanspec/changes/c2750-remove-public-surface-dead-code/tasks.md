# Tasks: c2750-remove-public-surface-dead-code

> 分诊方法见 design.md：编译器驱动三态（真死删 / 误标去 allow / 有理由保留须注释）。
> `skip_specs_landing: true`（纯 crate 内删除，无产品 MUST）。

## 1. 整文件级 allow

- [ ] 1.1 删 `agent/model/manifest.rs`、`infra/config/value.rs`、`infra/process/child.rs`（含 mod 声明与 re-export）。
- [ ] 1.2 `agent/prompt/skill_expand.rs` 逐项分诊：删死项、移除文件级 allow、保留活项。

## 2. 逐簇分诊（allow → warning → 三态）

- [ ] 2.1 c2750 标注簇：capabilities/mod.rs、context_policy/mod.rs、infra/event 第二总线方法面。
- [ ] 2.2 其余簇（clipboard / settings / trust / provider factory / image / ask / protocol helpers / model manager / stats / queue / freeze / mcp / shell / builder）。
- [ ] 2.3 bash.rs 特例：schema 字段理由注释保留核对；test-only seam 评估改 `#[cfg(test)]`。

## 3. 验证

- [ ] 3.1 `rg 'allow\(dead_code\)' src/` 余量逐条有注释理由；`just qa` + `just complexity`。
