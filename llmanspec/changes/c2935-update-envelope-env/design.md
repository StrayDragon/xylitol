# Design

## Decision

用户可设信封格式开关改名为 `XYLITOL_ENVELOPE_V3`，与代码标识 `ENVELOPE_FORMAT_ENV` 对齐。未设仍默认开。旧名不再读取。content-type `application/x-fory-v3` 与 fbs `namespace xy.wire.v3` 不动。

## Where

产品 TUI 信封客户端 `xylitol-app-tui` 的 `ENVELOPE_FORMAT_ENV` 一处读取。根包远端副本已删，无第二处。
