use std::collections::HashMap;
use std::hash::Hash;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Mcp,
    Lsp,
    Dap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceScope {
    Shared,
    Workspace,
    Session,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceKey {
    pub kind: ResourceKind,
    pub fingerprint: String,
    pub scope: ResourceScope,
    pub credential: Option<String>,
    pub workspace: Option<String>,
    pub session: Option<String>,
}

pub trait LeaseState {
    fn ref_count(&self) -> usize;
    fn set_ref_count(&mut self, count: usize);
    fn idle_generation(&self) -> u64;
    fn set_idle_generation(&mut self, generation: u64);
    fn idle_delay(&self) -> Duration;
    fn touch(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReleaseOutcome {
    pub generation: u64,
    pub delay: Duration,
}

pub struct ResourcePool<K, V> {
    entries: HashMap<K, V>,
    next_generation: u64,
}

impl<K, V> Default for ResourcePool<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            next_generation: 1,
        }
    }
}

impl<K: Eq + Hash, V: LeaseState> ResourcePool<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.entries.get_mut(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.entries.insert(key, value)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entries.remove(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.values()
    }

    pub fn into_values(self) -> impl Iterator<Item = V> {
        self.entries.into_values()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn acquire(&mut self, key: &K) -> Option<u64> {
        let generation = self.next_generation();
        let entry = self.entries.get_mut(key)?;
        entry.set_ref_count(entry.ref_count().saturating_add(1));
        entry.set_idle_generation(generation);
        entry.touch();
        Some(generation)
    }

    pub fn release(&mut self, key: &K) -> Option<ReleaseOutcome> {
        let generation = self.next_generation();
        let entry = self.entries.get_mut(key)?;
        let count = entry.ref_count().saturating_sub(1);
        entry.set_ref_count(count);
        entry.touch();
        if count > 0 {
            return None;
        }
        entry.set_idle_generation(generation);
        Some(ReleaseOutcome {
            generation,
            delay: entry.idle_delay(),
        })
    }

    fn next_generation(&mut self) -> u64 {
        let generation = self.next_generation;
        self.next_generation = self.next_generation.wrapping_add(1);
        generation
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    struct Entry {
        refs: usize,
        generation: u64,
        last_used: Instant,
        idle: Duration,
    }

    impl LeaseState for Entry {
        fn ref_count(&self) -> usize {
            self.refs
        }

        fn set_ref_count(&mut self, count: usize) {
            self.refs = count;
        }

        fn idle_generation(&self) -> u64 {
            self.generation
        }

        fn set_idle_generation(&mut self, generation: u64) {
            self.generation = generation;
        }

        fn idle_delay(&self) -> Duration {
            self.idle
        }

        fn touch(&mut self) {
            self.last_used = Instant::now();
        }
    }

    fn key(name: &str) -> ResourceKey {
        ResourceKey {
            kind: ResourceKind::Mcp,
            fingerprint: name.into(),
            scope: ResourceScope::Shared,
            credential: None,
            workspace: None,
            session: None,
        }
    }

    fn entry(idle_ms: u64) -> Entry {
        Entry {
            refs: 0,
            generation: 0,
            last_used: Instant::now(),
            idle: Duration::from_millis(idle_ms),
        }
    }

    #[test]
    fn acquire_increments_ref_count_and_bumps_generation() {
        let mut pool = ResourcePool::new();
        pool.insert(key("a"), entry(0));
        let first = pool.acquire(&key("a")).expect("entry exists");
        let second = pool.acquire(&key("a")).expect("entry exists");
        assert_ne!(first, second);
        assert_eq!(pool.get(&key("a")).map(|entry| entry.ref_count()), Some(2));
    }

    #[test]
    fn release_to_zero_returns_idle_outcome() {
        let mut pool = ResourcePool::new();
        pool.insert(key("a"), entry(50));
        pool.acquire(&key("a"));
        let outcome = pool.release(&key("a")).expect("last claim releases");
        assert_eq!(outcome.delay, Duration::from_millis(50));
        assert_eq!(pool.get(&key("a")).map(|entry| entry.ref_count()), Some(0));
    }

    #[test]
    fn release_while_claimed_returns_none() {
        let mut pool = ResourcePool::new();
        pool.insert(key("a"), entry(50));
        pool.acquire(&key("a"));
        pool.acquire(&key("a"));
        assert!(pool.release(&key("a")).is_none());
        assert_eq!(pool.get(&key("a")).map(|entry| entry.ref_count()), Some(1));
    }

    #[test]
    fn new_claim_invalidates_previous_idle_generation() {
        let mut pool = ResourcePool::new();
        pool.insert(key("a"), entry(50));
        pool.acquire(&key("a"));
        let released = pool.release(&key("a")).expect("release");
        let reused = pool.acquire(&key("a")).expect("reuse");
        assert_ne!(released.generation, reused);
    }

    #[test]
    fn insert_remove_and_len() {
        let mut pool = ResourcePool::new();
        pool.insert(key("a"), entry(0));
        assert_eq!(pool.len(), 1);
        assert!(!pool.is_empty());
        assert!(pool.remove(&key("a")).is_some());
        assert!(pool.is_empty());
    }
}
