//! Shared process/TLS observation gates + unit-test span collector.
//!
//! Provider plugins and compaction emit through the same atomics so composition
//! root / BDD scopes arm every crate at once.

use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use fastrace::collector::{Config, Reporter, SpanRecord};

/// Max Unicode scalars for `text` fields in provider-trace JSONL (c1000).
pub const PROVIDER_TRACE_TEXT_MAX: usize = 4096;

/// Hard cap for `observation_io = full` (protect OTLP batch size).
pub const OBSERVATION_IO_FULL_MAX: usize = 65_536;

/// Truncate for Langfuse observation I/O (shared by generation / tool / turn).
pub fn truncate_observation_text(text: &str, max: usize) -> (String, bool) {
    let count = text.chars().count();
    if count <= max {
        return (text.to_string(), false);
    }
    (text.chars().take(max).collect(), true)
}

static PROVIDER_TRACE_ACTIVE: AtomicBool = AtomicBool::new(false);
static OBSERVATION_IO_TIER: AtomicU8 = AtomicU8::new(0);
static TOOL_OBSERVATION_IO_TIER: AtomicU8 = AtomicU8::new(0);

/// How much generation / turn I/O to attach as Langfuse observation attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ObservationIoTier {
    #[default]
    None = 0,
    Truncated = 1,
    Full = 2,
}

impl ObservationIoTier {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Truncated,
            2 => Self::Full,
            _ => Self::None,
        }
    }

    /// Unicode-scalar cap for observation I/O at this tier (`None` → no I/O).
    pub fn max_chars(self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Truncated => Some(PROVIDER_TRACE_TEXT_MAX),
            Self::Full => Some(OBSERVATION_IO_FULL_MAX),
        }
    }
}

/// Snapshot of process-level obs gates for [`ObsGateScope`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObsGateState {
    pub active: bool,
    pub observation_io: ObservationIoTier,
    pub tool_observation_io: ObservationIoTier,
}

impl ObsGateState {
    pub const OFF: Self = Self {
        active: false,
        observation_io: ObservationIoTier::None,
        tool_observation_io: ObservationIoTier::None,
    };

    pub fn active_none_io() -> Self {
        Self {
            active: true,
            observation_io: ObservationIoTier::None,
            tool_observation_io: ObservationIoTier::None,
        }
    }
}

thread_local! {
    static SCOPED_GATES: RefCell<Option<ObsGateState>> = const { RefCell::new(None) };
}

/// RAII install of thread-local obs gates (unit tests / sync inject).
pub struct ObsGateScope {
    prev: Option<ObsGateState>,
}

impl ObsGateScope {
    pub fn enter(state: ObsGateState) -> Self {
        let prev = SCOPED_GATES.with(|c| c.borrow_mut().replace(state));
        Self { prev }
    }
}

impl Drop for ObsGateScope {
    fn drop(&mut self) {
        SCOPED_GATES.with(|c| {
            *c.borrow_mut() = self.prev.take();
        });
    }
}

fn with_gates_mut<R>(f: impl FnOnce(&mut ObsGateState) -> R) -> Option<R> {
    SCOPED_GATES.with(|c| {
        let mut g = c.borrow_mut();
        g.as_mut().map(f)
    })
}

/// Called from composition-root logging init when the FileReporter is installed.
pub fn set_provider_trace_active(active: bool) {
    if with_gates_mut(|s| s.active = active).is_some() {
        return;
    }
    PROVIDER_TRACE_ACTIVE.store(active, Ordering::Relaxed);
}

#[inline]
pub fn provider_trace_active() -> bool {
    if let Some(s) = SCOPED_GATES.with(|c| *c.borrow()) {
        return s.active;
    }
    PROVIDER_TRACE_ACTIVE.load(Ordering::Relaxed)
}

/// Called from composition root when `[otel].observation_io` is resolved.
pub fn set_observation_io_tier(tier: ObservationIoTier) {
    if with_gates_mut(|s| s.observation_io = tier).is_some() {
        return;
    }
    OBSERVATION_IO_TIER.store(tier as u8, Ordering::Relaxed);
}

#[inline]
pub fn observation_io_tier() -> ObservationIoTier {
    if let Some(s) = SCOPED_GATES.with(|c| *c.borrow()) {
        return s.observation_io;
    }
    ObservationIoTier::from_u8(OBSERVATION_IO_TIER.load(Ordering::Relaxed))
}

/// Called from composition root when `[otel].tool_observation_io` is resolved.
pub fn set_tool_observation_io_tier(tier: ObservationIoTier) {
    if with_gates_mut(|s| s.tool_observation_io = tier).is_some() {
        return;
    }
    TOOL_OBSERVATION_IO_TIER.store(tier as u8, Ordering::Relaxed);
}

#[inline]
pub fn tool_observation_io_tier() -> ObservationIoTier {
    if let Some(s) = SCOPED_GATES.with(|c| *c.borrow()) {
        return s.tool_observation_io;
    }
    ObservationIoTier::from_u8(TOOL_OBSERVATION_IO_TIER.load(Ordering::Relaxed))
}

static SPAN_SINK: Mutex<Option<std::sync::Arc<Mutex<Vec<SpanRecord>>>>> = Mutex::new(None);
static SPAN_COLLECT_EXCL: Mutex<()> = Mutex::new(());

struct DemuxReporter;

impl Reporter for DemuxReporter {
    fn report(&mut self, spans: Vec<SpanRecord>) {
        if let Some(buf) = SPAN_SINK.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            buf.lock().unwrap_or_else(|e| e.into_inner()).extend(spans);
        }
    }
}

fn ensure_demux_reporter() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        fastrace::set_reporter(DemuxReporter, Config::default());
    });
}

/// Unit-test span collector: installs a process demux reporter once.
pub struct SpanCollectScope {
    buf: std::sync::Arc<Mutex<Vec<SpanRecord>>>,
    prev: Option<std::sync::Arc<Mutex<Vec<SpanRecord>>>>,
    _excl: MutexGuard<'static, ()>,
}

impl SpanCollectScope {
    pub fn enter() -> Self {
        let excl = SPAN_COLLECT_EXCL.lock().unwrap_or_else(|e| e.into_inner());
        ensure_demux_reporter();
        let buf = std::sync::Arc::new(Mutex::new(Vec::new()));
        let prev = SPAN_SINK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .replace(std::sync::Arc::clone(&buf));
        Self {
            buf,
            prev,
            _excl: excl,
        }
    }

    pub fn records(&self) -> Vec<SpanRecord> {
        self.buf.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn records_for(&self, session_id: &str) -> Vec<SpanRecord> {
        self.records()
            .into_iter()
            .filter(|s| {
                s.properties
                    .iter()
                    .any(|(k, v)| k == "xylitol.session.id" && v == session_id)
            })
            .collect()
    }
}

impl Drop for SpanCollectScope {
    fn drop(&mut self) {
        *SPAN_SINK.lock().unwrap_or_else(|e| e.into_inner()) = self.prev.take();
    }
}
