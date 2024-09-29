use std::time::Duration;

use chrono::{DateTime, Utc};

/// Information about the freshness of a rendered response
#[derive(Debug, Clone, Copy)]
pub struct RenderFreshness {
    /// The age of the rendered response
    age: u64,
    /// The maximum age of the rendered response
    max_age: Option<u64>,
    /// The time the response was rendered
    timestamp: DateTime<Utc>,
}

impl RenderFreshness {
    /// Create new freshness information
    pub(crate) fn new(age: u64, max_age: u64, timestamp: DateTime<Utc>) -> Self {
        Self {
            age,
            max_age: Some(max_age),
            timestamp,
        }
    }

    /// Create new freshness information with only the age
    pub(crate) fn new_age(age: u64, timestamp: DateTime<Utc>) -> Self {
        Self {
            age,
            max_age: None,
            timestamp,
        }
    }

    /// Create new freshness information from a timestamp
    pub(crate) fn created_at(timestamp: DateTime<Utc>, max_age: Option<Duration>) -> Self {
        Self {
            age: timestamp
                .signed_duration_since(Utc::now())
                .num_seconds()
                .unsigned_abs(),
            max_age: max_age.map(|d| d.as_secs()),
            timestamp,
        }
    }

    /// Create new freshness information at the current time
    pub fn now(max_age: Option<Duration>) -> Self {
        Self {
            age: 0,
            max_age: max_age.map(|d| d.as_secs()),
            timestamp: Utc::now(),
        }
    }

    /// Get the age of the rendered response in seconds
    pub fn age(&self) -> u64 {
        self.age
    }

    /// Get the maximum age of the rendered response in seconds
    pub fn max_age(&self) -> Option<u64> {
        self.max_age
    }

    /// Get the time the response was rendered
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Write the freshness to the response headers.
    pub fn write(&self, headers: &mut http::HeaderMap<http::HeaderValue>) {
        let age = self.age();
        headers.insert(http::header::AGE, age.into());
        if let Some(max_age) = self.max_age() {
            headers.insert(
                http::header::CACHE_CONTROL,
                http::HeaderValue::from_str(&format!("max-age={}", max_age)).unwrap(),
            );
        }
    }
}
