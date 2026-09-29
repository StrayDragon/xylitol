# Dart 解码 smoke(c2834 task 4.3)

面向**主仓正式生成物**(`src/protocol/wire/v3/xy_wire_v3.fbs` → foryc `--lang dart`)
的跨语言冒烟:Rust 侧(同 fbs 的 generated.rs)编码帧 → Dart 解码。

## 复现(需 dart SDK;fory path 依赖本地 ../fory)

```bash
# 1) 重新生成 dart(可选,产物已 check-in)
PYTHONPATH=../../../../../../../fory/compiler python3 -c \
  "from fory_compiler.cli import main; import sys; sys.argv=['foryc', \
   '../../../../../../src/protocol/wire/v3/xy_wire_v3.fbs','--lang','dart','-o','/tmp/g']; main()"
cp /tmp/g/dart/xy/wire/v3/v3.dart lib/generated/v3.dart
dart pub get && dart run build_runner build
# 2) 解码 Rust 侧样本帧(由 /tmp/v3-full/smoke 式小 crate 生成,样本已 check-in)
dart run bin/decode.dart
# 预期:ServerNotification seq=128 / event=toolStart;ClientRequest rpc_id=7 request=describe
```

CI 接线受本地 ../fory 依赖限制,暂为手动维护命令(与 `just codegen-wire` 同性质)。
