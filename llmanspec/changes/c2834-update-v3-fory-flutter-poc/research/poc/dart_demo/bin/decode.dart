// 跨语言互操作 demo:解码 Rust 侧 fory 编码的 Frame 字节
import 'dart:io';
import 'dart:typed_data';
import 'package:dart_demo/generated/v3.dart';

void main(List<String> args) {
  final path = args.isEmpty ? '../frame_toolstart.bin' : args.first;
  final bytes = Uint8List.fromList(File(path).readAsBytesSync());
  final frame = Frame.fromBytes(bytes);
  final notif = frame.serverNotificationValue;
  print('seq = ${notif.seq}');
  final ev = notif.event;
  if (ev.isToolStart) {
    final ts = ev.toolStartValue;
    print('event = ToolStart id=${ts.id} name=${ts.name}');
    print('args_json = ${ts.argsJson}');
  } else {
    print('unexpected case: ${ev.caseValue}');
  }
}
