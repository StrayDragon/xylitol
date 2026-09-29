# c2834 research:线协议 v3(fory 二进制信封)调研与 PoC

> 基线快照:xylitol `main@f4b8d68e`(2026-09-29);apache/fory `963cb37`(2026-09-29,浅克隆单 commit,dev 1.8.0 / release 1.7.5)。
> 本目录为 change 作用域调研(AGENTS.md 约定),实现落地前不进 `docs/`。

## TL;DR

- **方向成立**:fdl 单源生成 + xlang 二进制编码,命中 Flutter 多端(Dart)前瞻的三项刚需——字段级版本演进(替代 `PROTOCOL_VERSION` 硬等值)、协议类型单源(替代 Rust 类型 + registry 测试锁的手工 schema)、大载荷体积(PoC 实测小消息即省 21%~50%)。
- **时机**:server-TUI(本机 loopback)是切换收益最小的场景,**现在不切**;触发点 = Flutter 端立项或远程场景落地。
- **PoC 已通全链路**:`.fdl` → foryc 生成 Rust + Dart → Rust roundtrip 冒烟 → **Rust 编码字节被 Dart 原生解码**(即 Flutter 客户端消费 host 下行帧的那条链路)。
- **三个硬结论**(纸面分析拿不到,见 03):
  1. fdl 保留字 `message` / `list` 撞现有高频字段名,v3 须全量盘点改名;
  2. `any` 类型不可用于任意 JSON(仅 bool/string/enum/message/union 且须两端注册),动态块唯一形态 = `string` 装原文;
  3. 生成物无 serde derive,双轨对拍期需要 fork foryc 加 `--rust-serde`(或上游 PR)。

## 阅读顺序

| 文档 | 内容 |
|---|---|
| [01-fory-评估.md](01-fory-评估.md) | Apache Fory 能力矩阵、成熟度证据、风险与退路(protobuf) |
| [02-xylitol-线协议现状盘点.md](02-xylitol-线协议现状盘点.md) | 现有 JSON-RPC 形态、强/动态分界线、接缝与被动面清单 |
| [03-poc-报告.md](03-poc-报告.md) | PoC 全过程与发现(fdl、生成物质量、roundtrip、跨语言、体积对比) |
| [04-迁移设计草案.md](04-迁移设计草案.md) | 信封设计、动态块策略、双轨对拍方案、决策记录(含被否方案) |

## PoC 复现(已在本目录验证)

前置:`apache/fory` clone 在 xylitol 同级(即 `../fory`);Rust(stable)与 Dart SDK(Flutter 自带即可)。

```bash
cd llmanspec/changes/c2834-update-v3-fory-flutter-poc/research/poc

# 0) 可选:从 fdl 重新生成(产物已归档,此步用于验证 compiler 可复现)
PYTHONPATH=../../../../../../../fory/compiler python3 -c \
  "from fory_compiler.cli import main; import sys; \
   sys.argv=['foryc','xy_wire.fdl','--lang','rust,dart','-o','generated']; main()"

# 1) Rust:编译生成物 + 3 场景 roundtrip + 写出跨语言字节帧
cd smoke && cargo run
# 预期输出:
#   scene1 ToolStart:   fory 173 B | json-rpc 219 B
#   scene2 MessageStart: fory 127 B | json-rpc 223 B
#   scene3 TodoList(30): fory 1267 B | json-rpc 2521 B
#   all roundtrips OK
# (并在 poc/ 写出 frame_toolstart.bin)

# 2) Dart:解码 Rust 编码的字节帧
cd ../dart_demo && dart pub get && dart run build_runner build \
  && dart run bin/decode.dart ../frame_toolstart.bin
# 预期输出:seq = 128 / event = ToolStart id=tool-42 name=grep / args_json = {...}
```

## 产物清单

```
poc/
├── xy_wire.fdl              # 协议切片 fdl(8 Event + 3 Command 变体 + Frame 信封)
├── generated/
│   ├── rust/xy_wire_v3.rs   # foryc 生成(599 行)
│   └── dart/xy/wire/v3/v3.dart  # foryc 生成(1132 行,注解骨架)
├── smoke/                   # Rust 冒烟(crate,path 依赖本地 ../fory/rust)
├── dart_demo/               # Dart 解码 demo(path 依赖本地 ../fory/dart)
│   └── lib/generated/v3.fory.dart  # build_runner 生成的 codec part
└── frame_toolstart.bin      # Rust 编码的跨语言样本帧(173 B)
```
