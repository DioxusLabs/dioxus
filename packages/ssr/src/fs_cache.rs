#![allow(non_snake_case)]

use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    time::Duration,
};

/// Information about the freshness of a rendered response
#[derive(Debug, Clone, Copy)]
pub struct RenderFreshness {
    /// The age of the rendered response
    age: u64,
    /// The maximum age of the rendered response
    max_age: Option<u64>,
}

impl RenderFreshness {
    /// Create new freshness information
    pub fn new(age: u64, max_age: u64) -> Self {
        Self {
            age,
            max_age: Some(max_age),
        }
    }

    /// Create new freshness information with only the age
    pub fn new_age(age: u64) -> Self {
        Self { age, max_age: None }
    }

    /// Create new freshness information at the current time
    pub fn now(max_age: Option<Duration>) -> Self {
        Self {
            age: 0,
            max_age: max_age.map(|d| d.as_secs()),
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

struct WriteBuffer {
    buffer: Vec<u8>,
}

impl std::fmt::Write for WriteBuffer {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buffer.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

impl Deref for WriteBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for WriteBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

pub(crate) struct ValidCachedPath {
    pub(crate) full_path: PathBuf,
    pub(crate) timestamp: std::time::SystemTime,
}

impl ValidCachedPath {
    pub fn try_from_path(value: PathBuf) -> Option<Self> {
        if value.extension() != Some(std::ffi::OsStr::new("html")) {
            return None;
        }
        let timestamp = decode_timestamp(value.file_stem()?.to_str()?)?;
        let full_path = value;
        Some(Self {
            full_path,
            timestamp,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn freshness(&self, max_age: Option<std::time::Duration>) -> Option<RenderFreshness> {
        let age = self.timestamp.elapsed().ok()?.as_secs();
        let max_age = max_age.map(|max_age| max_age.as_secs());
        Some(RenderFreshness::new(age, max_age?))
    }
}

fn decode_timestamp(timestamp: &str) -> Option<std::time::SystemTime> {
    let timestamp = u64::from_str_radix(timestamp, 16).ok()?;
    Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp))
}

pub fn timestamp() -> String {
    let datetime = std::time::SystemTime::now();
    let timestamp = datetime
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{:x}", timestamp)
}
