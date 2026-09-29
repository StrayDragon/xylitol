# 03 · PoC 报告:fdl → Rust/Dart 生成 → 跨语言互操作

> 目的:用真实工具链验证「xylitol 线协议 v3 可用 fdl 定义并生成双端代码」,拿到纸面分析得不到的实操结论。
> 产物见本目录 `poc/`;复现命令见 [README.md](README.md)(已在仓库内原位置复跑通过)。

## 1. 切片选择

不挑简单类型,专挑代表性与难啃的:

| 切片成分 | 代表什么 |
|---|---|
| `TextDelta` / `AgentEnd` | 最简流式增量 / unit 变体 |
| `ToolStart { args_json: string }` | **动态块**:工具参数 JSON 原文过线 |
| `ToolEnd { result: string, is_error: bool }` | 原文内嵌现成先例 + 严格必填 bool |
| `MessageStart { msg: optional AgentMessage }` | **被擦类型块**:AgentMessage 升级为判别 union(`LlmMessage | EnvMessage`,含 ContentPart 嵌套) |
| `CompactionEnd`(9 字段,6 optional) | 字段级演进语义代表(今天靠 `#[serde(default)]` 宽容解码) |
| `ContextTokenSettlement` | `usize` → `uint64` 的跨语言标量映射 |
| `TodoListSnapshot { todo_list: TodoList }` | 领域嵌套(struct + enum + list) |
| `Frame` union(`ClientRequest | ServerNotification`) | 信封:`rpc_id`/`method_id`/`seq` |

## 2. 流程实测

```bash
# 1) 生成(本地 fory/compiler,PYTHONPATH 直跑,零安装)
foryc xy_wire.fdl --lang rust,dart -o generated
# → rust/xy_wire_v3.rs(599 行)+ dart/xy/wire/v3/v3.dart(1132 行)

# 2) Rust 冒烟:include! 生成物 → 构造 → to_bytes/from_bytes → assert_eq
#    场景:ToolStart 帧 / MessageStart union / TodoList 30 条 —— 全过

# 3) Dart:build_runner 生成 codec part → 解码 Rust 写出的 173 字节帧
seq = 128
event = ToolStart id=tool-42 name=grep
args_json = {"include_hidden":false,"max_results":100,"path":"...","pattern":"TODO"}
```

## 3. 发现(按决策影响排序)

### 3.1 `any` 不可用于任意 JSON → 动态块唯一形态 = `string` 原文

schema-idl.md L1400-1425:动态值仅 `bool/string/enum/message/union`;**数字/list/map 不可**;具体类型必须两端注册,未知即失败。任意工具参数 JSON 无法用 `any` 表达。
→ v3 设计定案:动态块一律 `string` 装 JSON 原文(与 `ToolEnd.result` 现状同构)。对拍期该字段两条路径字节级一致;Dart 端 `json.decode` 一次。

### 3.2 fdl 保留字撞现有字段名 → v3 须全量盘点改名

实测两次撞车:`message`(ErrorEvent.detail / Prompt.text / MessageStart.msg 改名)与 `list`(TodoListSnapshot.todo_list 改名)。保留字表见 `compiler/fory_compiler/frontend/fdl/lexer.py` KEYWORDS(`message/list/map/array/option/service/rpc/stream/ref/...`)。
→ 正面:`type` **不**撞(生成器正确 emit `pub r#type`,Rust 关键字也处理好)。propose 时产出「全协议字段名 → v3 名」对照表。

### 3.3 生成物质量过硬,且有两处净收益

- 原生类型无 wrapper:`pub enum Event { #[fory(unknown)] Unknown(...), #[fory(id=1)] Error(ErrorEvent), ... }`;`optional` → `Option<T>` + `nullable=true`。
- **前向兼容载体内建**:`UnknownCase` 变体 + union case_id 判别(不是 untagged 试探)——`MessageStart.message` 今天被 `to_value` 擦成动态 + 形状试探解析,v3 升级为强 schema 判别式,Rust/Dart 两端受益。
- 运行时自洽:`OnceLock<Fory>` 全局单例(xlang + track_ref + compatible 全开),类型按 fdl `[id=N]` 注册,Dart 侧 `V3ForyModule.install(fory)` 同构。
- Dart union API 干净:`isXxx` / `xxxValue` getter + `toBytes/fromBytes`。

### 3.4 Dart 侧是两步生成(流程成本)

fdl 生成注解骨架(`v3.dart`,含 `part 'v3.fory.dart'` 声明)+ `dart run build_runner build` 生成 codec part(本机 17s)。官方 idl 测试同款流程(`integration_tests/idl_tests/run_dart_tests.sh`)。
→ CI 需加 build_runner 步骤;Flutter 工程内是标准姿势,无障碍。

### 3.5 生成物无 serde(再次印证)

`generators/rust.py` 全文无 serde;冒烟里 JSON 对比代码只能手写。
→ fork foryc 加 `--rust-serde`(f-string 模板,每类型加几行)支撑双轨对拍与持久化层复用;可作为上游 PR。

### 3.6 体积对比(同载荷,今天 JSON-RPC 形态 vs fory)

| 场景 | fory | json-rpc | 省 |
|---|---|---|---|
| ToolStart + 工具参数(动态块原文) | 173 B | 219 B | 21% |
| MessageStart(union 强 schema,2 content part) | 127 B | 223 B | 43% |
| TodoList 30 条(嵌套 struct+enum) | 1267 B | 2521 B | 50% |

小消息即 21%~50%;transcript 大载荷收益更大(重复 JSON key 名全部消失;注意本轮未做大载荷基准,正式决策前补线协议 benchmark)。

## 4. 未覆盖(后续阶段)

- evolving 双版本互操作(旧 schema 读新字段):引用官方 `integration_tests/idl_tests/idl/evolving*.idl` + 跨语言 CI 已验证的事实,未在本 PoC 自测;propose 前用 xylitol 真实类型做一组新旧 fdl 对拍。
- 38 Command 全量 + 服务端分发侧(本 PoC 只验证了数据面编解码,未动 salvo/jsonrpsee)。
- 性能(吞吐/延迟)基准——体积之外未测。
- `ref` 引用追踪(session tree 共享子树去重)未用,备用能力。
