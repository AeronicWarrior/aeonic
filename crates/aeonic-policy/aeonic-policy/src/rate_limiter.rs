use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

/// Simple in-process token bucket rate limiter.
/// One bucket per key (e.g. per provider or per API key).
///
/// For distributed deployments, replace with a Redis-backed implementation.
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<DashMap<String, Bucket>>,
    /// Max requests per window.
    capacity: u32,
    /// Window size in seconds.
    window_secs: u64,
}

#[derive(Debug, Clone)]
struct Bucket {
    tokens: u32,
    last_refill: i64,
}

impl RateLimiter {
    pub fn new(capacity: u32, window_secs: u64) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            capacity,
            window_secs,
        }
    }

    /// Try to consume one token for the given key.
    /// Returns true if allowed, false if rate limited.
    pub fn try_consume(&self, key: &str) -> bool {
        let now = Utc::now().timestamp();
        let mut bucket = self
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| Bucket {
                tokens: self.capacity,
                last_refill: now,
            });

        // Refill if window has elapsed
        let elapsed = (now - bucket.last_refill) as u64;
        if elapsed >= self.window_secs {
            bucket.tokens = self.capacity;
            bucket.last_refill = now;
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Remaining tokens for a key.
    pub fn remaining(&self, key: &str) -> u32 {
        self.buckets
            .get(key)
            .map(|b| b.tokens)
            .unwrap_or(self.capacity)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        // 1000 requests per 60 seconds per key by default
        Self::new(1000, 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumes_tokens_until_empty() {
        let limiter = RateLimiter::new(3, 60);
        assert!(limiter.try_consume("key1"));
        assert!(limiter.try_consume("key1"));
        assert!(limiter.try_consume("key1"));
        assert!(!limiter.try_consume("key1")); // exhausted
    }

    #[test]
    fn separate_keys_dont_interfere() {
        let limiter = RateLimiter::new(1, 60);
        assert!(limiter.try_consume("a"));
        assert!(!limiter.try_consume("a"));
        assert!(limiter.try_consume("b")); // b still has tokens
    }
}
