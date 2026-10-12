//! Wheel 管线 A/B 实验（c2891-followup MVP）：同壳三变体+延迟遥测。
//!
//! 300 行预置内容可滚；滚轮恒 ±3 直跳（无动画/加速——隔离管线变量）。
//! 退出时 stderr 汇总 wheel→paint 延迟 p50/p90/max 与帧距——「卡」客观化。
//!
//! | variant | 输入轮询 | wheel paint | 验证假设 |
//! |---|---|---|---|
//! | A | `poll(16ms)` 节拍 | 帧内（≈产品 EventStream 节奏） | 16ms cap 的钝感 |
//! | B | `poll(2ms)` 低延迟 | 帧内 | EventStream 异步链无罪论（纯同步更快?） |
//! | C | `poll(2ms)` | **每 wheel 事件即时渲**（零合帧） | 合帧/批处理罪 |
//!
//! Run: `cargo run -p xylitol-tui --example lab_wheel_pipeline -- --variant A [--raw]`
//! `--raw`=绕 xylitol diff 的纯 CSI 自绘（管线极限对照）；q/Ctrl+C 退出。

use std::io::{Write, stdout};
use std::time::{Duration, Instant};

use crossterm::event::Event as TermEvent;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use xylitol_tui::{ApplicationOwnedTui, Component, CrosstermTerminal, InputEvent};

const LINES: usize = 300;
const NOTCH: isize = 3;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Variant {
    A,
    B,
    C,
}

struct WheelHost {
    lines: Vec<String>,
    scroll: usize,
    /// 遥测：wheel 事件→render 完成延迟（ms）与 paint 间隔。
    lat: Vec<u128>,
    paint_gaps: Vec<u128>,
}

impl WheelHost {
    fn new() -> Self {
        let mut lines = Vec::with_capacity(LINES);
        for i in 0..LINES {
            // 长短混合+行号锚点，肉眼可辨滚动位移与顿挫。
            let pad = if i % 7 == 0 {
                "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod".to_string()
            } else {
                format!("{} row {i:03}", "· ".repeat(6))
            };
            lines.push(format!("{i:03} {pad}"));
        }
        Self {
            lines,
            scroll: 0,
            lat: Vec::new(),
            paint_gaps: Vec::new(),
        }
    }

    fn view_rows(&self, rows: usize) -> &[String] {
        let max = self.lines.len().saturating_sub(rows);
        let s = self.scroll.min(max);
        let e = (s + rows).min(self.lines.len());
        &self.lines[s..e]
    }

    fn wheel(&mut self, d: isize) {
        let max = self.lines.len() as isize - 1;
        self.scroll = (self.scroll as isize + d).clamp(0, max.max(0)) as usize;
    }
}

impl Component for WheelHost {
    fn render(&mut self, _width: usize) -> Vec<String> {
        // rows 由 terminal 高度决定；render 只拿 width——借用 viewport 高度的
        // 通用做法：给满行集，由 AO project 裁剪。此处直接交全部，代价可忽略。
        self.lines.clone()
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn input_wants_rerender(&self, _e: &InputEvent) -> bool {
        false
    }

    fn invalidate(&mut self) {}

    fn dock_rows_hint(&self) -> Option<usize> {
        Some(1)
    }
}

fn summary(host: &WheelHost, label: &str) {
    let mut lat = host.lat.clone();
    lat.sort_unstable();
    let p = |q: usize| lat.get(lat.len() * q / 100).copied().unwrap_or(0);
    let mut gaps = host.paint_gaps.clone();
    gaps.sort_unstable();
    let gp = |q: usize| gaps.get(gaps.len() * q / 100).copied().unwrap_or(0);
    eprintln!(
        "\n[telemetry:{label}] wheel_n={} lat_ms p50={} p90={} max={:?} | paint_gap_ms p50={} max={:?}",
        host.lat.len(),
        p(50),
        p(90),
        lat.last(),
        gp(50),
        gaps.last(),
    );
    eprintln!("[telemetry:{label}] all_lat={:?}", host.lat);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut variant = Variant::A;
    let mut raw = false;
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--raw" => raw = true,
            v => {
                if let Some(x) = v.strip_prefix("--variant=") {
                    variant = match x {
                        "A" => Variant::A,
                        "B" | "C" => Variant::B,
                        _ => Variant::A,
                    };
                    if x == "C" {
                        variant = Variant::C;
                    }
                }
            }
        }
    }
    let poll_ms = match variant {
        Variant::A => 16,
        Variant::B | Variant::C => 2,
    };
    eprintln!(
        "lab_wheel_pipeline variant={variant:?} poll={poll_ms}ms raw={raw} — 滚轮实验;q/Ctrl+C 退出,遥测见退出时 stderr"
    );

    if raw {
        return run_raw(poll_ms, variant);
    }

    let term = CrosstermTerminal::new()?;
    let _raw_guard = RawMode::new();
    let mut tui = ApplicationOwnedTui::new(term);
    tui.begin(); // begin 内含 terminal.start（alt+mouse）
    let host = Box::new(WheelHost::new());
    // host 双属：AO child+遥测接收——单所有权约束下用 Rc?example 从简：
    // 拆两结构会让遥测失真；此处 host 经 child 指针更新延迟不可行——
    // 改为全局遥测（静态），host 只滚。
    let mut lat: Vec<u128> = Vec::new();
    let mut paint_gaps: Vec<u128> = Vec::new();
    let mut paint_at: Option<Instant> = None;
    tui.add_child(host);
    tui.set_focus(Some(0));
    tui.set_dock_rows(1);
    tui.request_render(true);
    let _ = tui.render_now();

    loop {
        if event::poll(Duration::from_millis(poll_ms))? {
            let ev = event::read()?;
            match &ev {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.code == KeyCode::Char('q')
                    {
                        break;
                    }
                    // 双通道：tmux 等把滚轮本地译成 Up/Down——等价 wheel。
                    let d = match key.code {
                        KeyCode::Up => Some(-NOTCH),
                        KeyCode::Down => Some(NOTCH),
                        _ => None,
                    };
                    if let Some(d) = d {
                        let t0 = Instant::now();
                        let _ = tui.application_owned_scroll_by(d);
                        let _ = tui.render_now();
                        let ms = t0.elapsed().as_millis();
                        lat.push(ms);
                        let now = Instant::now();
                        if let Some(prev) = paint_at {
                            paint_gaps.push(now.duration_since(prev).as_millis());
                        }
                        paint_at = Some(now);
                    }
                }
                Event::Mouse(m)
                    if matches!(
                        m.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    ) =>
                {
                    let t0 = Instant::now();
                    let d = if matches!(m.kind, MouseEventKind::ScrollUp) {
                        -NOTCH
                    } else {
                        NOTCH
                    };
                    let _ = tui.application_owned_scroll_by(d);
                    let _ = tui.render_now();
                    let ms = t0.elapsed().as_millis();
                    lat.push(ms);
                    let now = Instant::now();
                    if let Some(prev) = paint_at {
                        paint_gaps.push(now.duration_since(prev).as_millis());
                    }
                    paint_at = Some(now);
                }
                TermEvent::Resize(..) => {
                    let _ = tui.render_now();
                }
                _ => {}
            }
            // 非 wheel key 路径的 rerender（q 已 break）
        } else if tui.idle_tick() {
            let _ = tui.try_render();
        }
    }
    let mut h = WheelHost::new();
    h.lat = lat.clone();
    h.paint_gaps = paint_gaps.clone();
    summary(&h, "app-owned");
    tui.finish();
    Ok(())
}

/// 纯 CSI 自绘：无 diff 引擎，直接重写视口——管线下限对照。
fn run_raw(poll_ms: u64, variant: Variant) -> Result<(), Box<dyn std::error::Error>> {
    let _raw_guard = RawMode::new();
    let mut host = WheelHost::new();
    let mut out = stdout();
    writeln!(out, "\x1b[?1049h\x1b[?1000h\x1b[?1006h")?;
    let (mut cols, mut rows) = (80, 24);
    if let Ok(sz) = terminal_size() {
        (cols, rows) = sz;
    }
    let _ = variant;
    let mut lat: Vec<u128> = Vec::new();
    let mut paint_gaps: Vec<u128> = Vec::new();
    let mut paint_at: Option<Instant> = None;
    draw_raw(&mut out, &host, cols, rows)?;
    loop {
        if event::poll(Duration::from_millis(poll_ms))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.code == KeyCode::Char('q')
                    {
                        break;
                    }
                    let d = match key.code {
                        KeyCode::Up => Some(-NOTCH),
                        KeyCode::Down => Some(NOTCH),
                        _ => None,
                    };
                    if let Some(d) = d {
                        let t0 = Instant::now();
                        host.wheel(d);
                        draw_raw(&mut out, &host, cols, rows)?;
                        let ms = t0.elapsed().as_millis();
                        lat.push(ms);
                        let now = Instant::now();
                        if let Some(prev) = paint_at {
                            paint_gaps.push(now.duration_since(prev).as_millis());
                        }
                        paint_at = Some(now);
                    }
                }
                Event::Resize(c, r) => {
                    (cols, rows) = (c as usize, r as usize);
                    draw_raw(&mut out, &host, cols, rows)?;
                }
                Event::Mouse(m)
                    if matches!(
                        m.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    ) =>
                {
                    let t0 = Instant::now();
                    let d = if matches!(m.kind, MouseEventKind::ScrollUp) {
                        -NOTCH
                    } else {
                        NOTCH
                    };
                    host.wheel(d);
                    draw_raw(&mut out, &host, cols, rows)?;
                    let ms = t0.elapsed().as_millis();
                    lat.push(ms);
                    let now = Instant::now();
                    if let Some(prev) = paint_at {
                        paint_gaps.push(now.duration_since(prev).as_millis());
                    }
                    paint_at = Some(now);
                }
                _ => {}
            }
        }
    }
    let mut h = WheelHost::new();
    h.lat = lat.clone();
    h.paint_gaps = paint_gaps.clone();
    summary(&h, "raw");
    write!(out, "\x1b[?1049l\x1b[?1006l\x1b[?1000l")?;
    out.flush()?;
    Ok(())
}

fn draw_raw(
    out: &mut impl Write,
    host: &WheelHost,
    cols: usize,
    rows: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let view = host.view_rows(rows.saturating_sub(1));
    let mut buf = String::from("\x1b[?2026h");
    for (i, line) in view.iter().enumerate() {
        buf.push_str(&format!("\x1b[{};1H\x1b[2K{}", i + 1, truncate(line, cols)));
    }
    buf.push_str("\x1b[?2026l");
    out.write_all(buf.as_bytes())?;
    out.flush()?;
    Ok(())
}

fn truncate(s: &str, cols: usize) -> String {
    s.chars().take(cols).collect()
}

fn terminal_size() -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let (c, r) = crossterm::terminal::size()?;
    Ok((c as usize, r as usize))
}

/// 进/退 raw mode（例内自管；RAII 退出恢复）。
struct RawMode;
impl RawMode {
    fn new() -> Self {
        let _ = crossterm::terminal::enable_raw_mode();
        Self
    }
}
impl Default for RawMode {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}
