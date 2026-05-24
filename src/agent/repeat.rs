//! Repeat detection and recovery module.
//!
//! Detects token repetition in LLM output streams using a sliding window
//! and n-gram HashSet. When repetition exceeds thresholds, the stream is
//! interrupted and recovery strategies are attempted.
//!
//! [MermaidChart:./docs/mmd/c35-repeat-detection.mmd]

#![allow(dead_code)] // WIP: not yet integrated into main flow

use std::collections::{HashSet, VecDeque};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tracing::warn;

use crate::infra::config::types::{RecoveryAction as RecoveryActionConfig, RecoveryConfig};

// ---------------------------------------------------------------------------
// DetectionConfig
// ---------------------------------------------------------------------------

/// Parameters for the n-gram repeat detection algorithm.
#[derive(Debug, Clone)]
pub(crate) struct DetectionConfig {
    /// Minimum n-gram length to check (default 3).
    pub min_n: u8,
    /// Maximum n-gram length to check (default 10).
    pub max_n: u8,
    /// Sliding window size in tokens (words).
    pub window_size: u16,
    /// Number of consecutive n-gram hits that triggers detection.
    pub consecutive_hit_threshold: u8,
    /// Fraction of window that must be repeats to trigger detection.
    pub window_repeat_ratio: f64,
    /// Stop monitoring after this many tokens (0 = no limit).
    pub early_stop_tokens: u16,
}

impl From<&crate::infra::config::types::RepeatDetectionConfig> for DetectionConfig {
    fn from(cfg: &crate::infra::config::types::RepeatDetectionConfig) -> Self {
        Self {
            min_n: cfg.min_n,
            max_n: cfg.max_n,
            window_size: cfg.window_size,
            consecutive_hit_threshold: cfg.consecutive_hit_threshold,
            window_repeat_ratio: cfg.window_repeat_ratio,
            early_stop_tokens: cfg.early_stop_tokens,
        }
    }
}

// ---------------------------------------------------------------------------
// DetectionResult
// ---------------------------------------------------------------------------

/// Information about a detected repetition.
#[derive(Debug, Clone)]
pub(crate) struct DetectionResult {
    /// Number of consecutive n-gram hits.
    pub consecutive_hits: u32,
    /// Fraction of the current window that consists of repeated content.
    pub window_repeat_ratio: f64,
}

// ---------------------------------------------------------------------------
// RepeatDetector
// ---------------------------------------------------------------------------

/// Streaming n-gram repeat detector.
///
/// Uses a sliding window of tokens (words split by whitespace) and an
/// n-gram HashSet to detect when the model starts repeating itself.
pub(crate) struct RepeatDetector {
    /// Sliding window of tokenized words.
    window: VecDeque<String>,
    /// Set of n-gram sequences seen so far.
    ngram_set: HashSet<Vec<String>>,
    /// Current consecutive n-gram hit count.
    consecutive_hits: u32,
    /// Configuration.
    config: DetectionConfig,
    /// Total tokens processed (for early_stop).
    total_tokens: u16,
    /// Whether early_stop has been reached.
    stopped: bool,
}

impl RepeatDetector {
    /// Create a new detector with the given configuration.
    pub(crate) fn new(config: DetectionConfig) -> Self {
        Self {
            window: VecDeque::with_capacity(config.window_size as usize),
            ngram_set: HashSet::new(),
            consecutive_hits: 0,
            config,
            total_tokens: 0,
            stopped: false,
        }
    }

    /// Feed a text chunk into the detector.
    ///
    /// Returns `Some(DetectionResult)` if repetition is detected, `None`
    /// otherwise. After a detection is returned, the detector enters a
    /// stopped state and will return `None` for subsequent calls.
    pub(crate) fn feed(&mut self, text: &str) -> Option<DetectionResult> {
        if self.stopped {
            return None;
        }

        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }

        for token in &tokens {
            self.total_tokens += 1;

            if self.config.early_stop_tokens > 0
                && self.total_tokens >= self.config.early_stop_tokens
            {
                self.stopped = true;
                return None;
            }

            self.window.push_back((*token).to_string());
            let window_len = self.window.len();

            // Check all n-gram suffixes from min_n to max_n.
            let mut any_hit = false;
            for n in self.config.min_n..=self.config.max_n {
                let n = n as usize;
                if window_len < n {
                    break;
                }
                let ngram: Vec<String> = self.window.range(window_len - n..).cloned().collect();
                if self.ngram_set.contains(&ngram) {
                    any_hit = true;
                }
            }

            if any_hit {
                self.consecutive_hits += 1;
            } else {
                self.consecutive_hits = 0;
                // Record all new n-grams for future comparison.
                for n in self.config.min_n..=self.config.max_n {
                    let n = n as usize;
                    if window_len < n {
                        break;
                    }
                    let ngram: Vec<String> = self.window.range(window_len - n..).cloned().collect();
                    self.ngram_set.insert(ngram);
                }
            }

            // Evict oldest token if window exceeds configured size.
            if self.window.len() > self.config.window_size as usize {
                self.window.pop_front();
            }

            // Check threshold: consecutive hits.
            if self.consecutive_hits >= self.config.consecutive_hit_threshold as u32 {
                self.stopped = true;
                let ratio = self.compute_window_repeat_ratio();
                return Some(DetectionResult {
                    consecutive_hits: self.consecutive_hits,
                    window_repeat_ratio: ratio,
                });
            }
        }

        None
    }

    /// Compute the fraction of the window that consists of repeated content.
    fn compute_window_repeat_ratio(&self) -> f64 {
        if self.window.is_empty() {
            return 0.0;
        }
        // Count tokens in the window that are part of any repeating n-gram.
        let window_len = self.window.len();
        let mut repeat_count = 0usize;

        for i in 0..window_len {
            for n in self.config.min_n..=self.config.max_n {
                let n = n as usize;
                if i + n > window_len {
                    break;
                }
                let ngram: Vec<String> = self.window.range(i..i + n).cloned().collect();
                if self.ngram_set.contains(&ngram) {
                    // All tokens in this n-gram are part of a repeat.
                    // Avoid double-counting by tracking tokens.
                    repeat_count += n;
                }
            }
        }

        // Normalize: a single n-gram hit shouldn't blow up the ratio.
        // Cap at window_len to keep ratio ∈ [0, 1].
        let repeat_count = repeat_count.min(window_len);
        repeat_count as f64 / window_len as f64
    }

    /// Reset the detector state for a new generation attempt.
    pub(crate) fn reset(&mut self) {
        self.window.clear();
        self.ngram_set.clear();
        self.consecutive_hits = 0;
        self.total_tokens = 0;
        self.stopped = false;
    }

    /// Whether the detector has been triggered or stopped.
    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped
    }
}

// ---------------------------------------------------------------------------
// RepeatDetectorStream
// ---------------------------------------------------------------------------

/// A stream wrapper that intercepts text content and checks for repetition.
///
/// Yields items from the inner stream transparently until repetition is
/// detected, after which the stream terminates (yields `None`).
pub(crate) struct RepeatDetectorStream<S> {
    inner: S,
    detector: RepeatDetector,
    detected: Option<DetectionResult>,
}

impl<S> RepeatDetectorStream<S> {
    /// Wrap a stream with repeat detection.
    pub(crate) fn new(inner: S, config: DetectionConfig) -> Self {
        Self {
            inner,
            detector: RepeatDetector::new(config),
            detected: None,
        }
    }

    /// Extract the detection result if repetition was found.
    pub(crate) fn detection_result(&self) -> Option<&DetectionResult> {
        self.detected.as_ref()
    }

    /// Consume and return the inner stream (e.g., for recovery retry).
    pub(crate) fn into_inner(self) -> S {
        self.inner
    }
}

impl<S, Item> Stream for RepeatDetectorStream<S>
where
    S: Stream<Item = Item> + Unpin,
    Item: AsRef<str>,
{
    type Item = Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.detected.is_some() {
            return Poll::Ready(None);
        }

        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(item)) => {
                let text: &str = item.as_ref();
                if let Some(result) = self.detector.feed(text) {
                    warn!(
                        consecutive_hits = result.consecutive_hits,
                        window_ratio = result.window_repeat_ratio,
                        "Repeat detection triggered, terminating stream"
                    );
                    self.detected = Some(result);
                    // Yield the last item (it triggered detection), then terminate.
                    Poll::Ready(Some(item))
                } else {
                    Poll::Ready(Some(item))
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// RecoveryManager
// ---------------------------------------------------------------------------

/// Manages sequential recovery actions when repeat detection triggers.
///
/// Implements the recovery strategy chain:
/// alter_prompt → switch_model → adjust_params → delegate_to_planner
pub(crate) struct RecoveryManager {
    config: RecoveryConfig,
    current_action: usize,
    attempts: u8,
}

/// The outcome of a single recovery action.
#[derive(Debug, Clone)]
pub(crate) enum RecoveryOutcome {
    /// Action executed successfully; caller should retry generation.
    Retry,
    /// All actions exhausted; caller should delegate to planner.
    Exhausted,
}

impl RecoveryManager {
    /// Create a new recovery manager from config.
    pub(crate) fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            current_action: 0,
            attempts: 0,
        }
    }

    /// Get the next recovery action to try.
    ///
    /// Returns `None` when all actions have been exhausted.
    pub(crate) fn next_action(&mut self) -> Option<&RecoveryActionConfig> {
        if self.attempts >= self.config.max_attempts {
            return None;
        }

        if self.current_action >= self.config.actions.len() {
            return None;
        }

        let action = &self.config.actions[self.current_action];
        self.attempts += 1;
        self.current_action += 1;
        Some(action)
    }

    /// Reset the recovery manager for a new detection cycle.
    pub(crate) fn reset(&mut self) {
        self.current_action = 0;
        self.attempts = 0;
    }

    /// Whether the recovery chain is exhausted.
    pub(crate) fn is_exhausted(&self) -> bool {
        self.attempts >= self.config.max_attempts
            || self.current_action >= self.config.actions.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use futures::stream;

    // ── RepeatDetector ────────────────────────────────────────────────

    fn default_detection_config() -> DetectionConfig {
        DetectionConfig {
            min_n: 3,
            max_n: 5,
            window_size: 50,
            consecutive_hit_threshold: 3,
            window_repeat_ratio: 0.8,
            early_stop_tokens: 0,
        }
    }

    #[test]
    fn test_detector_no_repetition() {
        let mut detector = RepeatDetector::new(default_detection_config());
        let text = "The quick brown fox jumps over the lazy dog";
        assert!(detector.feed(text).is_none(), "no repetition expected");
    }

    #[test]
    fn test_detector_detects_repetition() {
        let mut detector = RepeatDetector::new(DetectionConfig {
            consecutive_hit_threshold: 2,
            ..default_detection_config()
        });

        // With min_n=3, each 3-word sentence produces exactly 1 n-gram on
        // first encounter. A second feed of the same text gets a single hit
        // on the last word (consecutive=1, < threshold=2). The third feed
        // gets hits on the 1st word (consecutive re-establishes from the
        // n-grams inserted during feed 2), reaching threshold=2.
        assert!(
            detector.feed("aaa bbb ccc").is_none(),
            "feed 1: establish n-grams"
        );
        assert!(
            detector.feed("aaa bbb ccc").is_none(),
            "feed 2: hit on last word, consecutive=1"
        );
        assert!(
            detector.feed("aaa bbb ccc").is_some(),
            "feed 3: trigger on first word"
        );
    }

    #[test]
    fn test_detector_reset() {
        let mut detector = RepeatDetector::new(DetectionConfig {
            consecutive_hit_threshold: 2,
            ..default_detection_config()
        });

        // Same 3-word sentence pattern as above — triggers on feed 3.
        detector.feed("aaa bbb ccc");
        detector.feed("aaa bbb ccc");
        let result = detector.feed("aaa bbb ccc");
        assert!(result.is_some(), "repetition should be detected");

        // After reset, fresh state with empty window and ngram_set.
        detector.reset();
        assert!(!detector.is_stopped());

        // After reset, detection works again from scratch.
        assert!(detector.feed("aaa bbb ccc").is_none());
        assert!(detector.feed("aaa bbb ccc").is_none());
        assert!(
            detector.feed("aaa bbb ccc").is_some(),
            "repeats again after reset"
        );
    }

    #[test]
    fn test_detector_early_stop() {
        let mut detector = RepeatDetector::new(DetectionConfig {
            early_stop_tokens: 10,
            consecutive_hit_threshold: 5,
            ..default_detection_config()
        });

        // Feed 10 tokens (should stop after).
        detector.feed("a b c d e f g h i j");
        assert!(
            detector.is_stopped(),
            "detector should stop after 10 tokens"
        );
        assert!(
            detector.feed("k l m n o").is_none(),
            "detector should not check after stop"
        );
    }

    #[test]
    fn test_detector_empty_text() {
        let mut detector = RepeatDetector::new(default_detection_config());
        assert!(detector.feed("").is_none());
        assert!(detector.feed("   ").is_none());
    }

    #[test]
    fn test_detector_single_word_repeat() {
        let mut detector = RepeatDetector::new(DetectionConfig {
            min_n: 1,
            max_n: 2,
            consecutive_hit_threshold: 5,
            ..default_detection_config()
        });

        // Single repeated word — min_n=1 catches it.
        // Feed 1 = miss (first occurrence establishes the n-gram).
        // Feeds 2-6 = hits (consecutive_hits reaches 5 on feed 6).
        assert!(detector.feed("hello").is_none(), "feed 1: miss");
        assert!(
            detector.feed("hello").is_none(),
            "feed 2: hit, consecutive=1"
        );
        assert!(
            detector.feed("hello").is_none(),
            "feed 3: hit, consecutive=2"
        );
        assert!(
            detector.feed("hello").is_none(),
            "feed 4: hit, consecutive=3"
        );
        assert!(
            detector.feed("hello").is_none(),
            "feed 5: hit, consecutive=4"
        );
        assert!(
            detector.feed("hello").is_some(),
            "feed 6: trigger on consecutive=5"
        );
    }

    // ── RepeatDetectorStream ──────────────────────────────────────────

    #[tokio::test]
    async fn test_stream_passthrough() {
        let items = vec!["hello", "world", "foo", "bar"];
        let stream = stream::iter(items.clone());
        let detector_stream = RepeatDetectorStream::new(stream, default_detection_config());

        let collected: Vec<&str> = detector_stream.collect().await;
        assert_eq!(collected, items);
    }

    #[tokio::test]
    async fn test_stream_detects_repetition() {
        let items = vec![
            "the quick brown fox",
            "the quick brown fox",
            "the quick brown fox",
        ];
        let stream = stream::iter(items.clone());
        let detector_stream = RepeatDetectorStream::new(
            stream,
            DetectionConfig {
                consecutive_hit_threshold: 2,
                ..default_detection_config()
            },
        );

        let collected: Vec<&str> = detector_stream.collect().await;
        // Detection triggers on the 2nd item (hit on 2nd "the quick brown fox"
        // when n=4 suffix repeats). The trigger item is yielded, then stream
        // terminates.
        assert_eq!(collected.len(), 2, "should yield items 1 + trigger item 2");
    }

    #[tokio::test]
    async fn test_stream_no_detection_result_on_normal() {
        let items = vec!["hello world", "foo bar", "baz qux"];
        let stream = stream::iter(items.clone());
        let detector_stream = RepeatDetectorStream::new(stream, default_detection_config());

        let collected: Vec<&str> = detector_stream.collect().await;
        assert_eq!(collected.len(), 3);
    }

    // ── RecoveryManager ───────────────────────────────────────────────

    #[test]
    fn test_recovery_manager_exhaustion() {
        let config = RecoveryConfig {
            strategy: "sequential".into(),
            max_attempts: 3,
            actions: vec![
                RecoveryActionConfig::AlterPrompt {
                    prepend: "Avoid repetition.".into(),
                },
                RecoveryActionConfig::DelegateToPlanner,
            ],
        };

        let mut manager = RecoveryManager::new(config);

        assert!(manager.next_action().is_some());
        assert!(manager.next_action().is_some());
        assert!(manager.next_action().is_none(), "should be exhausted");
    }

    #[test]
    fn test_recovery_manager_reset() {
        let config = RecoveryConfig {
            strategy: "sequential".into(),
            max_attempts: 5,
            actions: vec![RecoveryActionConfig::AlterPrompt {
                prepend: "Avoid repetition.".into(),
            }],
        };

        let mut manager = RecoveryManager::new(config);
        assert!(manager.next_action().is_some());
        manager.reset();
        assert!(!manager.is_exhausted());
        assert!(manager.next_action().is_some());
    }

    #[test]
    fn test_recovery_manager_empty_actions() {
        let config = RecoveryConfig {
            strategy: "sequential".into(),
            max_attempts: 3,
            actions: vec![],
        };

        let mut manager = RecoveryManager::new(config);
        assert!(manager.next_action().is_none(), "no actions to try");
        assert!(manager.is_exhausted());
    }
}
