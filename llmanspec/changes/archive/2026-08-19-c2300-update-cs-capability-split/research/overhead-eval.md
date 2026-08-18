# 载体开销（lab/carrier_bench）

N=50k 帧、tagged JSON `{type,text}`、release。门槛：500 tok/s、p99 < 16ms。

| carrier | capacity | p99 |
|---|---|---|
| channel-raw | 3.50M fps | 0.05 µs |
| channel-json | 1.99M fps | 0.27 µs |
| tcp-json | 2.26M fps | 0.18 µs |
| bcast4-json | 321k fps | 0.48 µs |

容量 ≥ 门槛 640 倍。传输非主导；广播 clone 是相对大头。tagged JSON 足够，不预埋二进制 codec。复现：`c2300/lab/carrier_bench`，独立 `CARGO_TARGET_DIR`。
