# Design — c1215-add-test-bdd-solidify-pilot

## 背景

0.0.61 BDD-on 把 `spec.toon` scenarios 定为 SSOT，`solidify` 从中生成
`llmanspec/specs/<cap>/<cap>.feature`（场景标题 = `scenario.id`）。本项目现有 BDD 全走旧链路：
手写 `tests/features/*.feature`（中文场景标题）+ `bdd.rs` 绑定。两套链路标识空间、路径皆不同。

## 决策

1. **双轨并存**：试点不删旧链路（`tests/features/` + 中文标题绑定保持不动），新链路用新路径
   （`llmanspec/specs/<cap>/` + 英文 id 标题）。`validate` full mode 跑整批 `cargo test --test bdd`，
   两套场景同批通过即可。已验证 99 passed 零回归。

2. **手工生成 .feature**：`solidify` 只处理 change delta，对 main spec 无操作；而试点是在**已归档**
   的 `agent-runtime` main spec 上验证。故按 `solidify.rs:201-235` 的 `render_scenario`/`render_feature`
   规则手工产出 .feature，格式与工具产出字节级一致（`# language: zh-CN` + `假如/当/那么/而且`）。
   全量迁移时，新场景随 change delta 提交，`solidify` 会自动生成。

3. **step 文本 = spec.toon 字段**：新链路 step 函数文本直接取 spec.toon 的 given/when/then 自然语言
   字段（如 `mock 模型先 tool 后无 tool`）。这是 SSOT 单一性的代价：自然语言描述需逐条落 step
   实现。后续迁移可参数化（`{name}` 占位符）提升复用。

## 关键代码事实（支撑）

- `solidify` 写入路径硬编码 `llmanspec/specs/<cap>/<cap>.feature`（`src/sdd/solidify.rs:184-188`），
  不读 `config.feature_dir`。
- `locale_to_gherkin_lang`：`bdd.default_language: zh-CN` 优先（`solidify.rs:35-40`）→ 中文 Gherkin。
- `假如`（solidify）与 `假定`（现有 feature）均为合法 zh-CN Given，rstest-bdd 都识别，并存无冲突。

## 验证证据

- `cargo test --test bdd -- --test-threads=1` → 99 passed（97 旧 + 2 新）
- `llman sdd validate agent-runtime --check` → "1 feature(s) parsed, BDD check passed"

## 后续（B 阶段）

逐 spec 迁移，每 spec 一个独立 change（如 `c1220-migrate-domain-security-bdd`）。优先选旧 feature
已有 step 实现的 spec（摩擦最小）。
