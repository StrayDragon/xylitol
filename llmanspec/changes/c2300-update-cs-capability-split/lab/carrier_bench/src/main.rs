// Throwaway benchmark for c2300 "启动与连接方式" carrier decision (A: network CS vs B: in-process).
// Compares three carriers on a realistic TextDelta stream:
//   ① channel-raw   : in-process mpsc, NO serialization  (B direct upper bound)
//   ② channel-json  : in-process mpsc + tagged JSON ser+de (A-upper / B-with-contract)
//   ③ tcp-json      : loopback TCP + length-prefixed tagged JSON (A real transport; WS adds only a few header bytes + mask)
//   broadcast4-json : ② with fan-out to 4 consumers (tests the "broadcast dominates" claim)
// See research/overhead-eval.md for the model + gate.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

const N: usize = 50_000;

#[derive(serde::Serialize, serde::Deserialize)]
struct TextDelta {
    #[serde(rename = "type")]
    r#type: String,
    text: String,
}

fn fixture_texts() -> Vec<String> {
    (0..N).map(|i| "x".repeat(1 + (i % 40) as usize)).collect()
}

fn to_json(text: &str) -> String {
    serde_json::to_string(&TextDelta {
        r#type: "text_delta".into(),
        text: text.into(),
    })
    .unwrap()
}

fn parse_json(json: &str) {
    let _: TextDelta = serde_json::from_str(json).unwrap();
}

fn percentile(mut v: Vec<Duration>, p: f64) -> Duration {
    v.sort();
    let idx = ((v.len() as f64 - 1.0) * p).round() as usize;
    v[idx.clamp(0, v.len() - 1)]
}

fn report(name: &str, wall: Duration, per: Vec<Duration>) {
    let mean_us = per.iter().map(|d| d.as_secs_f64() * 1e6).sum::<f64>() / per.len() as f64;
    let fps = N as f64 / wall.as_secs_f64();
    let p50 = percentile(per.clone(), 0.50).as_secs_f64() * 1e6;
    let p95 = percentile(per.clone(), 0.95).as_secs_f64() * 1e6;
    let p99 = percentile(per.clone(), 0.99).as_secs_f64() * 1e6;
    println!(
        "| {name:<14} | {wall:>9.2} ms | {fps:>11.0} fps | {mean_us:>7.2} µs | {p50:>7.2} | {p95:>7.2} | {p99:>7.2} |>",
        wall = wall.as_secs_f64() * 1e3
    );
}

fn carrier_channel_raw(texts: &[String]) {
    let texts = texts.to_vec();
    let (tx, rx) = mpsc::channel::<String>();
    let t0 = Instant::now();
    let prod = thread::spawn(move || {
        for t in &texts {
            tx.send(t.clone()).unwrap();
        }
    });
    let cons = thread::spawn(move || {
        let mut per = Vec::with_capacity(N);
        while let Ok(t) = rx.recv() {
            let st = Instant::now();
            let _ = t.len();
            per.push(st.elapsed());
        }
        per
    });
    prod.join().unwrap();
    let per = cons.join().unwrap();
    report("channel-raw", t0.elapsed(), per);
}

fn carrier_channel_json(texts: &[String]) {
    let texts = texts.to_vec();
    let (tx, rx) = mpsc::channel::<String>();
    let t0 = Instant::now();
    let prod = thread::spawn(move || {
        for t in &texts {
            tx.send(to_json(t)).unwrap();
        }
    });
    let cons = thread::spawn(move || {
        let mut per = Vec::with_capacity(N);
        while let Ok(j) = rx.recv() {
            let st = Instant::now();
            parse_json(&j);
            per.push(st.elapsed());
        }
        per
    });
    prod.join().unwrap();
    let per = cons.join().unwrap();
    report("channel-json", t0.elapsed(), per);
}

fn write_frame(sock: &mut TcpStream, bytes: &[u8]) -> std::io::Result<()> {
    let len = bytes.len() as u32;
    sock.write_all(&len.to_le_bytes())?;
    sock.write_all(bytes)?;
    sock.flush()
}

fn read_exact_or_eof(sock: &mut TcpStream, buf: &mut [u8]) -> std::io::Result<bool> {
    let mut got = 0;
    while got < buf.len() {
        let n = sock.read(&mut buf[got..])?;
        if n == 0 {
            if got == 0 {
                return Ok(false);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "mid-frame eof",
            ));
        }
        got += n;
    }
    Ok(true)
}

fn carrier_tcp(texts: &[String]) -> std::io::Result<()> {
    let texts = texts.to_vec();
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let t0 = Instant::now();
    let server = thread::spawn(move || -> std::io::Result<Vec<Duration>> {
        let (mut sock, _) = listener.accept()?;
        let mut per = Vec::with_capacity(N);
        let mut lenb = [0u8; 4];
        let mut buf = Vec::with_capacity(512);
        while read_exact_or_eof(&mut sock, &mut lenb)? {
            let len = u32::from_le_bytes(lenb) as usize;
            buf.resize(len, 0);
            read_exact_or_eof(&mut sock, &mut buf)?;
            let st = Instant::now();
            parse_json(std::str::from_utf8(&buf).unwrap());
            per.push(st.elapsed());
        }
        Ok(per)
    });
    let client = thread::spawn(move || -> std::io::Result<()> {
        let mut sock = TcpStream::connect(addr)?;
        for t in &texts {
            let j = to_json(t);
            write_frame(&mut sock, j.as_bytes())?;
        }
        sock.shutdown(std::net::Shutdown::Write)?;
        Ok(())
    });
    let per = server.join().unwrap()?;
    client.join().unwrap()?;
    report("tcp-json", t0.elapsed(), per);
    Ok(())
}

fn carrier_broadcast4(texts: &[String]) {
    const NCONS: usize = 4;
    let texts = texts.to_vec();
    let (txs, rxs): (Vec<Sender<String>>, Vec<Receiver<String>>) =
        (0..NCONS).map(|_| mpsc::channel()).unzip();
    let t0 = Instant::now();
    let prod = thread::spawn(move || {
        for t in &texts {
            let j = to_json(t);
            for tx in &txs {
                tx.send(j.clone()).unwrap();
            }
        }
    });
    let cons: Vec<_> = rxs
        .into_iter()
        .map(|rx| {
            thread::spawn(move || {
                let mut per = Vec::with_capacity(N);
                while let Ok(j) = rx.recv() {
                    let st = Instant::now();
                    parse_json(&j);
                    per.push(st.elapsed());
                }
                per
            })
        })
        .collect();
    let mut all = Vec::with_capacity(N * NCONS);
    for c in cons {
        all.extend(c.join().unwrap());
    }
    prod.join().unwrap();
    report(&format!("bcast{}-json", NCONS), t0.elapsed(), all);
}

fn main() {
    let texts = fixture_texts();
    println!(
        "c2300 载体开销评估 (N={N} frames, text 1..40 chars, tagged JSON {{type,text}}, release)"
    );
    println!("| carrier | total wall | capacity | avg/frame | p50 µs | p95 µs | p99 µs |");
    println!("|---|---|---|---|---|---|---|");
    carrier_channel_raw(&texts);
    carrier_channel_json(&texts);
    carrier_broadcast4(&texts);
    carrier_tcp(&texts).unwrap();
    println!();
    println!("判读指引：门槛基准 = 500 tok/s（≈2ms/帧），p99 上限 ~16ms，CPU 增量 ~5%。");
    println!("- channel-raw → channel-json 差 = serde 代价；channel-json → tcp-json 差 = 网络/传输代价。");
    println!("- 若 tcp-json 容量 ≫ 500 fps 且 p99 ≪ 16ms：传输非主导 → 统一 CS(A) 可作默认。");
}
