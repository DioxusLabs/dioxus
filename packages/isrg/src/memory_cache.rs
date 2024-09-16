//! Incremental file based incremental rendering

#![allow(non_snake_case)]

use chrono::offset::Utc;
use chrono::DateTime;
use rustc_hash::FxHasher;
use std::{hash::BuildHasherDefault, num::NonZeroUsize};

use super::freshness::RenderFreshness;

pub(crate) struct InMemoryCache {
    #[allow(clippy::type_complexity)]
    lru: Option<lru::LruCache<String, (DateTime<Utc>, Vec<u8>), BuildHasherDefault<FxHasher>>>,
    invalidate_after: Option<std::time::Duration>,
}

impl InMemoryCache {
    pub fn new(memory_cache_limit: usize, invalidate_after: Option<std::time::Duration>) -> Self {
        Self {
            lru: NonZeroUsize::new(memory_cache_limit)
                .map(|limit| lru::LruCache::with_hasher(limit, Default::default())),
            invalidate_after,
        }
    }

    pub fn clear(&mut self) {
        if let Some(cache) = &mut self.lru {
            cache.clear();
        }
    }

    pub fn put(&mut self, route: String, timestamp: DateTime<Utc>, data: Vec<u8>) {
        if let Some(cache) = &mut self.lru {
            cache.put(route, (timestamp, data));
        }
    }

    pub fn invalidate(&mut self, route: &str) {
        if let Some(cache) = &mut self.lru {
            cache.pop(route);
        }
    }

    pub fn try_get_or_insert<'a, F: FnOnce() -> Result<(DateTime<Utc>, Vec<u8>), E>, E>(
        &'a mut self,
        route: &str,
        or_insert: F,
    ) -> Result<Option<(RenderFreshness, &'a [u8])>, E> {
        if let Some(memory_cache) = self.lru.as_mut() {
            let (timestamp, _) = memory_cache.try_get_or_insert(route.to_string(), or_insert)?;

            let now = Utc::now();
            let elapsed = timestamp.signed_duration_since(now);
            let age = elapsed.num_seconds();
            // The cache entry is out of date, so we need to remove it.
            if let Some(invalidate_after) = self.invalidate_after {
                if elapsed.to_std().unwrap() > invalidate_after {
                    tracing::trace!("memory cache out of date");
                    memory_cache.pop(route);
                    return Ok(None);
                }
            }

            // We need to reborrow because we may have invalidated the lifetime if the route was removed.
            // We know it wasn't because we returned... but rust doesn't understand that.
            let (timestamp, cache_hit) = memory_cache.get(route).unwrap();

            return match self.invalidate_after {
                Some(invalidate_after) => {
                    tracing::trace!("memory cache hit");
                    let max_age = invalidate_after.as_secs();
                    let freshness = RenderFreshness::new(age as u64, max_age, *timestamp);
                    Ok(Some((freshness, cache_hit)))
                }
                None => {
                    tracing::trace!("memory cache hit");
                    let freshness = RenderFreshness::new_age(age as u64, *timestamp);
                    Ok(Some((freshness, cache_hit)))
                }
            };
        }

        Ok(None)
    }
}
