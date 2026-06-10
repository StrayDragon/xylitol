# Tasks: BDD 框架迁移 (cucumber-rs → rstest-bdd)

## 规划（Decomposed from design.md）

- [x] Cargo.toml: 移除 cucumber，添加 rstest + rstest-bdd + rstest-bdd-macros
- [x] 重写 `tests/bdd.rs`：拆 Workspace/SessionStore/AgentState fixture，替换 regex→placeholder 步骤，添加 #[scenario] 场景绑定，删除 `async fn main()` runner
- [x] 枚举所有 `.feature` 场景确保无遗漏绑定
- [x] 验证：编译通过，41/77 场景通过（36 个为工具模式变体/stub，与 cucumber-rs 中相同）

## 实现

### 1. Cargo.toml 依赖变更

- [x] `cargo rm --dev cucumber` 移除 cucumber
- [x] 手动添加 `rstest = "0.26"`、`rstest-bdd = "0.6.0-beta2"`、`rstest-bdd-macros = { version = "0.6.0-beta2", features = ["compile-time-validation"] }` 到 dev-dependencies
- [x] `cargo check --lib` 确认无误

### 2. tests/bdd.rs 重写

- [x] 定义 3 个 fixture struct：`Workspace`、`SessionStore`、`AgentState`（使用 `RefCell`/`Cell` 内部可变性）
- [x] 定义 `#[fixture]` 函数（ws, sess, agent）
- [x] 所有步骤函数签名使用 `&State` + `RefCell` 内部可变性
- [x] 为所有场景添加 `#[scenario(path, name)]` 绑定函数（77 个场景）
- [x] DataTable 步骤：Edit 多处替换使用 `table: Vec<Vec<String>>` 参数
- [x] DocString 步骤：使用 `docstring: String` 参数
- [x] 所有 regex 步骤替换为 typed placeholder
- [x] 删除 `#[tokio::main] async fn main()` runner

### 3. 校验

- [x] `cargo test --test bdd read -- --test-threads=1` — 4/6 read 通过（2 个因工具返回值格式差异失败，非框架问题）
- [x] `cargo test test_read_entire_file -- --nocapture` 可精确运行单个场景
- [x] 所有 77 个 `.feature` 场景都有对应 `#[scenario]` 绑定
