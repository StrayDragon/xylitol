# Tasks — c2899 packages 情态词清整

- [x] T1 脚本清整：映射表（design）应用 18 feature；描述行/步骤行分型；生成 diff 供人工 review。
- [x] T2 步骤行↔bindings 同步核查：改动的场景步骤 grep 主 crate steps 文本，逐一修正并 `cargo test --test bdd`。
- [x] T3 结构门：`llman-sdd validate --all-roots --strict`（staleness 除外）；残留英文情态计数归零复核。
