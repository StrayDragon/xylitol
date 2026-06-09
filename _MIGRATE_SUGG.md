# 从 cucumber-rs 迁移到 rstest-bdd 推荐指南

> 2026-06-09 · 基于 rstest-bdd 0.6.0-beta2 + cucumber-rs 0.21 的对拍实验撰写
> 目标读者：pytest-bdd 背景的 Rust 开发者

---

## 为什么要考虑迁移？

### 你可能正在忍受的 cucumber-rs 痛点

1. **自定义 runner** — 必须写 `async fn main() { World::run().await }`，脱离 `cargo test` 体系
2. **World 单体 struct** — 所有场景共享一个巨大 struct，字段互相污染
3. **regex-only 步骤匹配** — `(\d+)` `([^"]+)` 难读难维护，无类型标注
4. **运行时才发现缺失步骤** — CI 跑了好几分钟才报 "step not found"
5. **无法 `cargo test test_login`** — 自定义 runner，IDE 点击运行不生效
6. **无法混合单元测试** — BDD 测试必须独立二进制，和单元测试割裂

### rstest-bdd 的核心价值

```
pytest-bdd 的 Rust 版——如果你用过 pytest-bdd，你已经会了。
```

| 概念 | pytest-bdd | rstest-bdd | cucumber-rs |
|------|-----------|-----------|-------------|
| 步骤装饰器 | `@given` / `@when` / `@then` | `#[given]` / `#[when]` / `#[then]` | `#[given]` / `#[when]` / `#[then]` |
| 参数捕获 | `{name:d}` `{text}` | `{name:i64}` `{text}` | `(\d+)` `(.+)` regex |
| 场景绑定 | `@scenario("file", "name")` | `#[scenario(path, name)]` | ❌ 自动发现 |
| 状态注入 | `@pytest.fixture` | `#[fixture]` | `World` struct |
| 测试运行器 | `pytest` | `cargo test` | 自定义 `World::run()` |
| 步骤验证 | 运行时 | **编译时** | 运行时 |

**学习曲线 ≈ 学 Rust 语法，零框架概念学习成本。**

---

## 概念映射速查表

### World → Fixture

```rust
// ❌ cucumber-rs: 一个巨大的 World
#[derive(Debug, Default, World)]
pub struct World {
    result: i64,
    text: String,
    cart_total: f64,
    user_name: String,
    user_age: u32,
    async_result: i64,
}

// ✅ rstest-bdd: 按场景按需组合，互不污染
#[derive(Debug, Default)]
struct CalcState { result: i64 }

#[derive(Debug, Default)]
struct CartState { total: f64 }

#[fixture]
fn calc() -> CalcState { CalcState::default() }

#[fixture]
fn cart() -> CartState { CartState::default() }
```

### 步骤定义

```rust
// ❌ cucumber-rs: regex，位置绑定
#[when(regex = r"I add (\d+) and (\d+)")]
fn when_add(world: &mut World, a: i64, b: i64) {
    world.result = a + b;
}
//       位置1↑       位置2↑  ← 需要对照函数签名

// ✅ rstest-bdd: typed placeholder，名称内联
#[when("I add {a:i64} and {b:i64}")]
fn when_add(calc: &CalcState, a: i64, b: i64) {
    calc.result.set(a + b);
}
//       名称↑类型↑    名称↑类型↑  ← 步骤模式本身就是文档
```

### 场景绑定

```rust
// ❌ cucumber-rs: 无显式绑定，目录扫描自动执行
// 你不知道哪些场景被测了，无法精确运行单个场景

// ✅ rstest-bdd: 显式绑定，精确控制
#[scenario(path = "features/login.feature", name = "成功登录")]
#[test]
fn test_login_success(calc: CalcState) {}

// 精确运行: cargo test test_login_success
```

### Data Tables

```rust
// ❌ cucumber-rs: 通过 Step 手动访问
#[given("the following items:")]
fn given_items(world: &mut World, step: &Step) {
    if let Some(table) = &step.table {
        for row in table.rows.iter().skip(1) {
            let price: f64 = row[1].parse().unwrap();
            // ...
        }
    }
}

// ✅ rstest-bdd: 直接作为函数参数注入
#[given("the following items:")]
fn given_items(calc: &CalcState, #[datatable] table: Vec<Vec<String>>) {
    for row in table.iter().skip(1) {
        let price: f64 = row[1].parse().unwrap();
        // ...
    }
}
```

### Doc Strings

```rust
// ❌ cucumber-rs: 通过 Step 手动访问
#[when("I process the following text:")]
fn when_process(world: &mut World, step: &Step) {
    if let Some(docstring) = &step.docstring {
        world.text = docstring.clone();
    }
}

// ✅ rstest-bdd: 直接作为 String 参数
#[when("I process the following text:")]
fn when_process(calc: &CalcState, docstring: String) {
    *calc.text.borrow_mut() = docstring;
}
```

### Async 步骤

```rust
// 两者语法几乎相同
// cucumber-rs:
#[given("an async calculator")]
async fn given_async(_world: &mut World) {
    sleep(Duration::from_millis(10)).await;
}

// rstest-bdd (需添加 use std::future::Future;):
#[given("an async calculator")]
async fn given_async(_calc: &CalcState) {
    sleep(Duration::from_millis(10)).await;
}
```

---

## 迁移步骤

### Step 1: 修改 Cargo.toml

```toml
# 删除
# cucumber = { version = "0.21", features = ["timestamps"] }
# [[test]]
# name = "bdd"
# harness = false

# 添加
[dev-dependencies]
rstest = "0.24"
rstest-bdd = "0.6.0-beta2"
rstest-bdd-macros = { version = "0.6.0-beta2", features = ["strict-compile-time-validation"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }  # 如果需要 async
```

> `strict-compile-time-validation` 会在编译时报错缺失步骤，而非运行时才发现。
> 如果觉得太严格，降级为 `compile-time-validation`（警告）或去掉（运行时报错）。

### Step 2: 重写状态管理

```rust
// 将 World struct 拆分为多个独立 fixture

// 如果 fixture 需要跨步骤共享可变状态，用 Cell/RefCell：
use std::cell::{Cell, RefCell};

#[derive(Debug, Default)]
struct MyState {
    value: Cell<i64>,
    name: RefCell<String>,
    data: RefCell<Vec<String>>,
}

#[fixture]
fn state() -> MyState { MyState::default() }
```

> **为什么需要 Cell/RefCell？** rstest-bdd 的步骤函数接收 `&State`（不可变引用），
> 因为 `&mut State` 与 typed placeholder 捕获参数会触发借用冲突。
> 用内部可变性是当前 beta 版的 workaround。

### Step 3: 将 regex 步骤改为 typed placeholder

```
regex                        →  typed placeholder
─────────────────────────────────────────────────
(\d+)                        →  {n:i64}
(-?\d+)                      →  {n:i64}
([\d.]+)                     →  {n:f64}
(\S+)                        →  {word}
"([^"]+)"                    →  {text:string}  (feature 文件需保留引号)
(.+)                         →  {text}         (默认匹配到空格)
```

### Step 4: 重写步骤函数签名

```rust
// cucumber-rs:
#[given(regex = r#"a user named "([^"]+)" with age (\d+)"#)]
fn given_user(world: &mut World, name: String, age: u32) { ... }

// rstest-bdd:
#[given("a user named {name:string} with age {age:u32}")]
fn given_user(state: &MyState, name: String, age: u32) { ... }
```

### Step 5: 为每个场景添加 `#[scenario]` 绑定

```rust
// cucumber-rs: 不需要（自动发现）
// rstest-bdd: 显式绑定

#[scenario(path = "features/login.feature", name = "成功登录")]
#[test]
fn test_login_success(state: MyState) {}

#[scenario(path = "features/login.feature", name = "密码错误")]
#[test]
fn test_login_wrong_password(state: MyState) {}

// async 场景:
#[scenario(path = "features/api.feature", name = "异步请求")]
#[tokio::test]
async fn test_async_request(state: MyState) {}
```

### Step 6: 删除 `async fn main()` runner

```rust
// 删除整个 runner:
// #[tokio::main]
// async fn main() {
//     World::cucumber()
//         .run_and_exit("features/")
//         .await;
// }
```

### Step 7: 运行

```bash
# 之前 (cucumber-rs):
cargo test --test bdd

# 之后 (rstest-bdd):
cargo test                                    # 跑全部
cargo test test_login_success                 # 跑单个场景
cargo test login                              # 按名称过滤
cargo test -- --nocapture                     # 看完整输出
cargo test -- --test-threads=1                # 串行执行
```

---

## 已知限制与 Workaround

### 1. regex 不工作

**现状：** `(\d+)` `(.+)` 等 regex 捕获组在 0.6.0-beta2 中不匹配。

**Workaround：** 使用 typed placeholder `{name:Type}`，这也是更可读的写法。

**趋势：** rstest-bdd 的设计哲学就是 typed placeholder 优先，regex 是给 cucumber-rs 迁移用的兼容接口。如需 regex，可保留 `expr = "..."` 语法待后续版本修复。

### 2. `&mut` fixture + 捕获参数 = 借用冲突

**现状：** 步骤函数不能同时有 `&mut State` 和 typed placeholder 参数。

**Workaround：** 使用 `Cell<T>` / `RefCell<T>` 包装可变字段，步骤函数接收 `&State`。

```rust
#[derive(Debug, Default)]
struct State {
    count: Cell<i64>,       // 简单值用 Cell
    items: RefCell<Vec<String>>,  // 复杂值用 RefCell
}
```

### 3. 中文 Examples 列名不支持

**现状：** `| 商品 | 价格 |` 中文列名无法被 `<商品>` 引用。

**Workaround：** 列名用英文，步骤文本保持中文：

```gherkin
# language: zh-CN
功能: 购物车
  场景大纲: 添加商品
    当 我添加 "{name}" 单价 {price} 数量 {qty}

    例子:
      | name | price | qty |
      | 苹果 | 2.5   | 4   |
```

### 4. Rule 内场景无法绑定

**现状：** `Rule` 块内的场景名称无法被 `#[scenario]` 发现。

**Workaround：** 将 Rule 内场景提到 Feature 顶层，或改用 Tags 组织。

### 5. gherkin 0.14.0 部分 Scenario Outline 解析失败

**现状：** 部分 Scenario Outline 文件在 gherkin 0.14.0 下解析失败（cucumber-rs 同样受影响）。

**Workaround：** 如遇解析失败，简化 Examples 表或检查格式。中文版 Scenario Outline（`场景大纲`）通常能正常解析。

---

## typed placeholder 支持的类型

| 类型提示 | 匹配规则 | 示例 |
|---------|---------|------|
| `u8` `u16` `u32` `u64` `u128` `usize` | `\d+` | `{count:u32}` → `42` |
| `i8` `i16` `i32` `i64` `i128` `isize` | `[+-]?\d+` | `{val:i32}` → `-5` |
| `f32` `f64` | 浮点数（含科学计数法） | `{price:f64}` → `2.5` |
| `string` | 引号字符串，自动去引号 | `{name:string}` → `"Alice"` → Alice |
| _(其他)_ | `.+?` 非贪婪，解析为 String | `{text}` → 任意文本到空格 |

> **多词值必须用 `{name:string}` 并在 feature 文件中加引号。**
> `{item}` 只匹配到第一个空格。

---

## 迁移检查清单

- [ ] Cargo.toml: 替换 cucumber 为 rstest + rstest-bdd
- [ ] 删除 `[[test]] name = "bdd" harness = false`
- [ ] 删除 `async fn main()` runner
- [ ] World struct → 独立 fixture struct（+ Cell/RefCell）
- [ ] regex 步骤 → typed placeholder
- [ ] 步骤函数 `&mut World` → `&State`
- [ ] 添加 `use std::future::Future;`（如果有 async 步骤）
- [ ] 为每个场景添加 `#[scenario]` + `#[test]`
- [ ] 场景函数声明 fixture 参数 `fn test_xxx(state: MyState)`
- [ ] Data Tables: `step.table` → `#[datatable] table: Vec<Vec<String>>`
- [ ] Doc Strings: `step.docstring` → `docstring: String`
- [ ] Examples 列名改为英文（如果有中文列名）
- [ ] `cargo test` 验证全部通过

---

## 附录：对拍实验数据

实验项目保存在 `.tmp/bdd-compare/`，包含：

- `shared-features/` — 10 个共享 feature 文件
- `cucumber-demo/` — cucumber-rs 完整实现
- `rstest-bdd-demo/` — rstest-bdd 完整实现
- `REPORT.md` — 详细对比报告

| 实验 | cucumber-rs | rstest-bdd |
|------|:-----------:|:----------:|
| E01 基本 Given/When/Then | ✅ | ✅ |
| E02 中文 Feature | ✅ | ✅ |
| E03 Scenario Outline | ⚠️ gherkin bug | ⚠️ gherkin bug |
| E04 Background | ⚠️ gherkin bug | ⚠️ gherkin bug |
| E05 Data Tables | ✅ | ✅ |
| E06 Doc Strings | ✅ | ✅ |
| E07 Tags | ✅ | ✅ |
| E07 Rule | ✅ | ❌ |
| E08 Regex/Typed 捕获 | ✅ regex | ✅ placeholder |
| E09 Async | ✅ | ✅ |
| E10 中文 Outline+Background | ✅ | ⚠️ 列名英文 |
