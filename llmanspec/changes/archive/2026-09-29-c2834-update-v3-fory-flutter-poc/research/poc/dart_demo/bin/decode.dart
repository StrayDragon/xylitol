// 跨语言互操作 smoke(c2834 task 4.3):解码主仓正式 v3 生成物编码的帧。
// 用法:dart run bin/decode.dart [frame.bin ...](默认解码本目录样本)。
import 'dart:io';
import 'package:dart_demo/generated/v3.dart';

void main(List<String> args) {
  final paths = args.isEmpty
      ? ['frame_notification.bin', 'frame_request.bin']
      : args;
  for (final path in paths) {
    final bytes = File(path).readAsBytesSync();
    final frame = Frame.fromBytes(bytes);
    switch (frame.caseValue) {
      case FrameCase.serverNotification:
        final n = frame.serverNotificationValue;
        print('$path: ServerNotification seq=${n.seq}');
        if (n.notification.isEvent) {
          final ev = n.notification.eventValue;
          print('  event=${ev.caseValue}');
        }
      case FrameCase.clientRequest:
        final r = frame.clientRequestValue;
        print('$path: ClientRequest rpc_id=${r.rpcId} '
            'request=${r.request.caseValue}');
      default:
        print('$path: unexpected ${frame.caseValue}');
    }
  }
}
