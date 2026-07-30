//! Generic in-memory cache for a single expiring token-like value, shared
//! by each provider's token/credential manager (Spotify client-credentials
//! and user tokens, Apple Music developer tokens).

use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Whether an expiring value is still usable, i.e. not within
/// `refresh_margin` of `expires_at`. Exposed standalone (not just via
/// `TokenCache`) for managers that need to hold a value past its
/// freshness window — e.g. to read a refresh token off a stale access
/// token — without going through the cache's own locking.
pub(crate) fn is_fresh(expires_at: Instant, refresh_margin: Duration) -> bool {
    match expires_at.checked_sub(refresh_margin) {
        Some(effective_expiry) => Instant::now() < effective_expiry,
        None => false,
    }
}

pub(crate) struct TokenCache<T: Clone> {
    inner: RwLock<Option<(T, Instant)>>,
    /// How long before actual expiry to treat the cached value as stale,
    /// so callers have time to refresh before a request would fail.
    refresh_margin: Duration,
}

impl<T: Clone> TokenCache<T> {
    pub fn new(refresh_margin: Duration) -> Self {
        Self {
            inner: RwLock::new(None),
            refresh_margin,
        }
    }

    /// Returns the cached value if present and not within `refresh_margin`
    /// of its expiry.
    pub async fn get(&self) -> Option<T> {
        let guard = self.inner.read().await;
        guard.as_ref().and_then(|(value, expires_at)| {
            if is_fresh(*expires_at, self.refresh_margin) {
                Some(value.clone())
            } else {
                None
            }
        })
    }

    pub async fn set(&self, value: T, ttl: Duration) {
        *self.inner.write().await = Some((value, Instant::now() + ttl));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_fresh_true_well_before_expiry() {
        let expires_at = Instant::now() + Duration::from_secs(3600);
        assert!(is_fresh(expires_at, Duration::from_secs(60)));
    }

    #[test]
    fn is_fresh_false_within_refresh_margin() {
        let expires_at = Instant::now() + Duration::from_secs(30);
        assert!(!is_fresh(expires_at, Duration::from_secs(60)));
    }

    #[test]
    fn is_fresh_false_when_margin_exceeds_time_since_epoch() {
        // expires_at is "now", so subtracting any positive margin
        // underflows — must not panic, must read as stale.
        assert!(!is_fresh(Instant::now(), Duration::from_secs(60)));
    }

    #[tokio::test]
    async fn empty_cache_returns_none() {
        let cache: TokenCache<String> = TokenCache::new(Duration::from_secs(60));
        assert_eq!(cache.get().await, None);
    }

    #[tokio::test]
    async fn returns_cached_value_before_expiry() {
        let cache = TokenCache::new(Duration::from_secs(60));
        cache
            .set("token".to_string(), Duration::from_secs(3600))
            .await;
        assert_eq!(cache.get().await, Some("token".to_string()));
    }

    #[tokio::test]
    async fn treats_value_as_stale_within_refresh_margin() {
        let cache = TokenCache::new(Duration::from_secs(60));
        // Expires in 30s, but the refresh margin is 60s, so it should
        // already read as stale to give callers time to refresh.
        cache
            .set("token".to_string(), Duration::from_secs(30))
            .await;
        assert_eq!(cache.get().await, None);
    }
}
