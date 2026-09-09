//! Fan-out fastrace [`Reporter`] — delivers the same batch to zero or more sinks.

use fastrace::collector::{Reporter, SpanRecord};

/// Forwards each report batch to every inner reporter (File, OTEL, …).
pub struct FanoutReporter {
    inner: Vec<Box<dyn Reporter>>,
}

impl FanoutReporter {
    pub fn new(inner: Vec<Box<dyn Reporter>>) -> Self {
        Self { inner }
    }

    pub fn push(&mut self, reporter: impl Reporter) {
        self.inner.push(Box::new(reporter));
    }

    pub fn push_box(&mut self, reporter: Box<dyn Reporter>) {
        self.inner.push(reporter);
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Reporter for FanoutReporter {
    fn report(&mut self, spans: Vec<SpanRecord>) {
        match self.inner.split_last_mut() {
            None => {}
            Some((last, rest)) => {
                for reporter in rest {
                    reporter.report(spans.clone());
                }
                last.report(spans);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct CollectReporter(Arc<Mutex<usize>>);

    impl Reporter for CollectReporter {
        fn report(&mut self, spans: Vec<SpanRecord>) {
            *self.0.lock().unwrap() += spans.len();
        }
    }

    #[test]
    fn fanout_delivers_to_all() {
        let a = Arc::new(Mutex::new(0usize));
        let b = Arc::new(Mutex::new(0usize));
        let mut fan = FanoutReporter::new(vec![
            Box::new(CollectReporter(a.clone())),
            Box::new(CollectReporter(b.clone())),
        ]);
        fan.report(vec![]);
        // empty batch still calls report
        assert_eq!(*a.lock().unwrap(), 0);
        assert_eq!(*b.lock().unwrap(), 0);
    }
}
