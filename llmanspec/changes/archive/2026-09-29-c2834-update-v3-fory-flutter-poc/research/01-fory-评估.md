# 01 · Apache Fory 评估

> 证据来源:本地浅克隆 `../fory`(HEAD `963cb37`,2026-09-29)。浅克隆无法统计提交频率,活跃度结论以 CHANGELOG / 版本 / CI 配置为证。
> 行号基于该 commit,后续上游演进可能漂移,以路径为准。

## 1. 它是什么

Apache 顶级项目(已毕业,非孵化器;`NOTICE` 全库无 incubating 标注,官网 fory.apache.org)。多语言序列化框架 + IDL 编译器,**不带传输层**(纯序列化;RPC 仅提供 gRPC marshaller/stub 代码生成,`docs/grpc/index.md`)。

三层能力:

| 层 | 说明 |
|---|---|
| wire format | xlang 跨语言二进制(默认)、native(语言私有对象图)、Standard Row Format(零拷贝随机访问) |
| runtime | 各语言包(Rust `fory`/`fory-core`/`fory-derive`,Dart `fory`,等 11 语言) |
| compiler | PyPI 包 `fory-compiler`(CLI `foryc`),fdl/.proto/.fbs 三前端 → 11 语言代码生成 |

## 2. 成熟度(按语言,与 xylitol 相关的两端)

### Rust —— 成熟,已发 crates.io

- workspace `rust/Cargo.toml`:1.8.0-alpha.0(dev),crates.io 已发布 **1.7.5**(`rust/README.md` badge);MSRV 1.70。
- **xlang 是 Rust 默认模式**(`rust/README.md` L9),跨语言场景零额外配置。
- proc-macro:`fory-derive`(`ForyStruct`/`ForyEnum`/`ForyUnion`/`ForyRow`)。
- native 模式支持 `Arc/Rc/ArcWeak/RcWeak` 循环引用、trait 对象、tuple(测试齐全,`rust/tests/tests/` 48 个测试文件)——xylitol 用不上但说明运行时深度。
- 演进语义(`rust/README.md` L340-344):字段按名匹配可重排、可增删(缺省默认值)、可改 nullability,**不可改类型**。
- CI:`.github/workflows/ci.yml` L1249「Rust CI」、L1284「Rust Xlang Test」(与 Java 字节级互操作)、L955 Java/Rust gRPC;发布流水线 `release-rust.yaml` 走 `cargo publish`。

### Dart —— 成熟,1.0 后持续发布

- `dart/packages/fory/pubspec.yaml`:1.8.0-dev;pub.dev 已发布 **1.7.5**(与 Rust 同号,天然对齐)。CHANGELOG:0.17-dev → 1.0.0 → … → 1.7.5,1.5.0 有 breaking 先例。
- **仅支持 xlang**(README L374 明示)——而 fdl 生成的恰好就是 xlang 代码,与 xylitol 用法(host=Rust, client=Dart)完全落在支持象限。
- 声明支持 Flutter VM / AOT / web(README L9-20);代码生成走 `build_runner`(注解路径),fdl 路径生成注解骨架后仍需 build_runner 出 codec part(见 03 §3.4)。
- CI:`ci.yml` L1966「Dart CI」(VM 测试 + AOT/JS smoke)、L1085 Java/Dart gRPC。

### compiler —— 功能完整但自标 Alpha

- `compiler/pyproject.toml` classifier `Development Status :: 3 - Alpha`;**零运行时依赖**(纯 Python,直接 PYTHONPATH 即可跑,PoC 已验证)。
- 生成器为手写 Python f-string 模板(`generators/rust.py` 1742 行、`dart.py` 1406 行),fork 成本低——这正是「加 `--rust-serde`」可行性的依据。
- 25 个 pytest 文件 + 独立 Compiler CI job;`integration_tests/idl_tests/` 有 11 语言落盘与互操作脚本。

## 3. wire format 关键性质(xlang)

规范:`docs/specification/xlang_serialization_spec.md`(1971 行)。布局 = `fory header | object ref meta | object type meta | object value data`,小端。

| 性质 | 语义 | 对 xylitol 的意义 |
|---|---|---|
| 动态自描述 | compatible 模式流内写完整 TypeDef,后续出现写引用;读端按 name/tag 匹配,**未知字段跳过** | 前后向兼容 = 字段级演进;抓包可解(自描述),补偿二进制调试面损失 |
| 双注册 | numeric ID(`[id=N]`)或 name-based;fdl 省略 id 时编译器用 MurmurHash3(package.type) 自动分配并查重 | 方法 ID/类型 ID 可全部编译期分配 |
| 引用追踪 | 每对象 ref flag(NULL/REF/NOT_NULL/REF_VALUE),支持共享去重与循环引用 | session tree / 重复子树可用 `ref` 表达(PoC 未用,备用) |
| unknown fields | **不保留**(与 protobuf 的差异点) | xylitol 现状同样不保留,无损失 |
| same-schema mode | 4 字节 schema hash 硬校验 | 不采用;走 compatible |

**`any` 的真实限制**(schema-idl.md L1400-1425,决策关键):动态值仅允许 `bool/string/enum/message/union`;**数字、bytes、日期、list/map 不支持**;且具体类型必须两端注册,未知类型反序列化失败。结论:**任意 JSON 不可能用 `any` 表达**,动态块唯一形态是 `string` 原文(见 04 §2)。

## 4. fdl(fory definition language)速览

- protobuf 风格:`message/enum/union/service` + 字段编号 + `[id=N]` 类型选项 + `reserved`。手写 lexer + 递归下降 parser(不依赖 ANTLR)。
- 演进规则:字段编号 `0 ≤ n < 2^29` 且 message 内唯一;删除字段须 `reserved` 保留编号/名,永不复用;`evolving` 选项切换 compatible/same-schema。
- 完整文法:schema-idl.md L1690-1752。
- **保留字撞车**(PoC 实测):`message`、`list`、`map`、`array`、`option`、`service`、`rpc`、`stream` 等均为关键字,不能做字段名(见 03 §3.2)。

## 5. 与退路方案对比(protobuf)

需求拆解 = 「IDL 单源 + Rust/Dart 生成 + 字段级演进 + 二进制」:

| 维度 | fory | protobuf(prost + dart protobuf) |
|---|---|---|
| 生态成熟度 | 年轻(1.x),compiler Alpha | 事实标准,最 boring |
| 性能 | 官方 benchmark 约 10×(自家口径) | 慢但够用 |
| 生成类型 | 原生类型,无 wrapper | Dart 侧有生成包装类 |
| 引用追踪/循环引用 | 一等公民 | 无 |
| unknown fields | 不保留 | 保留 |
| serde 共存 | 生成物无 serde(fork 可加) | prost 生态有 pbjson 等桥接 |
| 多端 Flutter web | 声明支持 | 支持 |

**结论**:fory 作为首选评估(形状完全对口:host-Rust/多端-Dart 恰好都是其成熟象限),protobuf 为退路。若阶段评估(PoC 扩展到全量协议)发现生成质量或上游迭代不可接受,切换成本主要在 fdl→proto 的机械翻译。

## 6. 风险清单

| 风险 | 证据 | 缓解 |
|---|---|---|
| compiler Alpha / 上游 breaking | dart 1.5.0 breaking 先例 | 锁 release 1.7.5 + fork vendor 意向 + 字节 conformance 快照测试 |
| 锁版本 = 全端捆绑同一 fory 运行时 | xlang codec 同版本假设 | 格式稳定期内字段演进仍可独立;维护「协议支持矩阵」心智 |
| 生成物无 serde | `generators/rust.py` 全文无 serde | fork 加 `--rust-serde`(模板加几行)或上游 PR |
| 文档 benchmark 口径自利 | 性能数据多为自家 | PoC 实测体积;性能基准进 xylitol 线协议 benchmark(待建) |
