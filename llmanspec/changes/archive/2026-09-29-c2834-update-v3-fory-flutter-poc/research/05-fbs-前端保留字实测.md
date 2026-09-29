# 05 · 保留字逃逸:官方出路实测(fbs 前端)

> 问题:fdl 保留字(`message`/`list`/`timestamp` 等)撞现有 wire 字段名(04 §2.5,11 处)。
> 本篇实测 fory 官方三条路,**结论:改用 fbs(FlatBuffers schema)前端做协议真源,字段名零改名**。
> 产物:`poc/fbs_escape/`;fory 基线同 `963cb37`。

## 1. 三条路实测结论

| 路径 | 结果 | 证据 |
|---|---|---|
| fdl 原生 | ✗ 无字段名转义机制 | lexer 将保留字归为 keyword token,parser 字段名位置 `consume(IDENT)` 严格(`frontend/fdl/parser.py:336` 类);无反引号/引号字段名语法 |
| fdl `json_name` 选项 | ✗ 死选项 | `KNOWN_FIELD_OPTIONS` 接受它(`fdl/parser.py:65`)但 **11 个生成器零引用**(grep `json_name generators/` 为空);schema-idl.md 亦无文档 |
| proto 前端 | ✗ 实测拒绝 | `string message = 2;` → `Error: t.proto:5:10: Expected field name`(实测输出);fory 的 proto parser 与 fdl 同样严格,**比真 protoc 更严**(protoc 接受 message 作字段名) |
| **fbs 前端** | ✓ **实测通过** | 关键字表(`frontend/fbs/lexer.py:91`)不含 message/list/map/timestamp/optional/ref/string;见下 |

## 2. fbs 前端实测(完整链路)

```fbs
namespace xy.wire.v3;
enum TodoStatus : int { Pending, InProgress, Completed }
table ErrorEvent { kind: string; message: string; }   // fdl 保留字,fbs 合法
union Event { ErrorEvent, ToolStart, ToolEnd }
table ServerNotification {
    seq: ulong; event: Event; list: string; timestamp: long;
}
```

- `foryc xy_wire.fbs --lang rust,dart` 编译通过;
- 生成物**字段名原名保留**:Rust `pub message: ::std::string::String`、`pub list`、`pub timestamp`;Dart `String message`;wire TypeDef 字段名同 IR 原名 → **与今日 JSON 线字段名逐字段一致**;
- Rust roundtrip 实测通过(94 B,含 message/list/timestamp 三个原保留名字段,断言相等);
- 产物:`poc/fbs_escape/`(.fbs + 生成物 + smoke,复现命令同 README §PoC,将 fdl 换 fbs 即可)。

## 3. 代价与差异(vs fdl 真源)

| 维度 | fdl | fbs | 对 v3 的影响 |
|---|---|---|---|
| 类型 ID | 显式 `[id=N]` 或 auto-hash | **仅 auto**(MurmurHash(namespace.类型名)) | ID 稳定性 = 类型名稳定性;类型不改名即可;重命名视为删旧加新 |
| 字段编号 | 显式编号,自由分配 | **声明顺序(从 1 起)**,无显式编号 | 演进纪律变化:新字段只能**尾部追加**;删字段保留占位、编号永不复用(FlatBuffers 成熟惯例;v3 全新编号,初始无影响) |
| union case | 自定义 case 名 + 显式 ID | case 名从类型名派生、ID 声明顺序 | 变体名即类型名(`Event::ErrorEvent`),少一层命名自由度,语义等价 |
| optional | `optional T` | 标量默认非空,须逐字段 attribute `(fory_nullable:true)` | `Option<String>` 类字段要逐个标 attribute,略繁琐 |
| map | `map<K,V>` | **不支持**(translator 仅 FbsVectorType) | xylitol wire 层仅 1 个 map 字段(`model_entry.rs:39` `thinking_level_map`),用 keyed vector(table + `(key)` 属性)表达,消费端一行转 HashMap |
| ref 引用追踪 | `ref` 修饰符 | attribute `fory_ref` | 语义可达,不损失 |
| evolving | per-type 选项 | table→evolving=true / struct→false,自动 | v3 全部用 table,即全演进 |

**结论**:对 xylitol 的协议面,fbs 前端表达力足够(唯一缺口 1 个 map 字段,有惯例解法),换来 11 处字段名零改名 + wire 字段名与现有 JSON 线逐字段一致(对拍两路径的 wire 语义直接对齐,连 D6 的「对拍点设领域对象层」兜底都少了一层需求)。**修订决策:协议真源从 fdl 改为 fbs**(04 §2.5 的改名表作废,保留为 fdl 路径的历史备选)。

## 4. 残留风险

- fbs 前端同为 Alpha compiler 的一部分,坑位与 fdl 相同(conformance 快照测试同等覆盖);
- 声明顺序即编号:`git diff` 审查 schema 时插入位置错误会被 conformance 快照测试捕获(字节变化),纪律靠 review + 测试;
- 若未来 fdl 加了字段名转义(可向上游提 issue/PR),可迁回 fdl;迁移成本 = 语法翻译(ID 语义对齐),wire 字节不变(字段名/编号一致)。
