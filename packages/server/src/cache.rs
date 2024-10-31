use std::path::PathBuf;

use chrono::{DateTime, Utc};
use http::HeaderValue;
use quick_cache::{
    sync::{Cache, DefaultLifecycle},
    DefaultHashBuilder, OptionsBuilder, UnitWeighter,
};

use crate::RenderChunk;

pub struct CachedPages {
    cache: Cache<String, CachedPage, UnitWeighter>,
}

impl CachedPages {
    fn new() -> Self {
        //  Cache::<(String, u64), String>::with_options(
        //    OptionsBuilder::new()
        //      .estimated_items_capacity(10000)
        //      .weight_capacity(10000)
        //      .build()
        //      .unwrap(),
        //      UnitWeighter,
        //      DefaultHashBuilder::default(),
        //      DefaultLifecycle::default(),
        //  );

        Self {
            cache: Cache::new(100),
        }
    }

    pub fn get(&self, route: &str) -> Option<CachedPage> {
        self.cache.get(route)
    }
}

#[derive(Clone)]
pub struct CachedPage {
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub max_age: Option<chrono::Duration>,
    pub fs_entry: Option<PathBuf>,
}

impl CachedPage {
    pub fn is_fresh(&self) -> bool {
        !self.is_stale()
    }

    pub fn is_stale(&self) -> bool {
        let Some(max_age) = self.max_age else {
            return false;
        };

        self.age() > max_age
    }

    pub fn age(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.timestamp)
    }

    pub fn to_chunk(&self) -> RenderChunk {
        let mut chunk = RenderChunk::from_contents(self.content.clone());
        chunk
            .headers
            .insert(http::header::AGE, self.age().num_seconds().into());
        if let Some(max_age) = self.max_age {
            chunk.headers.insert(
                http::header::CACHE_CONTROL,
                HeaderValue::from_str(&format!("max-age={}", max_age.num_seconds()))
                    .expect("Max age header to be valid"),
            );
        }

        chunk
    }
}
