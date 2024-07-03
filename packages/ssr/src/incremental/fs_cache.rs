#![allow(non_snake_case)]

use chrono::{DateTime, Utc};

use super::{IncrementalRendererError, RenderFreshness};
use std::{path::PathBuf, sync::Arc, time::SystemTime};

pub(crate) type PathMapFn = Arc<dyn Fn(&str) -> PathBuf + Send + Sync>;

pub(crate) struct FileSystemCache {
    static_dir: PathBuf,
    map_path: PathMapFn,
    invalidate_after: Option<std::time::Duration>,
}

impl FileSystemCache {
    pub fn new(
        static_dir: PathBuf,
        map_path: Option<PathMapFn>,
        invalidate_after: Option<std::time::Duration>,
    ) -> Self {
        Self {
            static_dir: static_dir.clone(),
            map_path: map_path.unwrap_or_else(move || {
                Arc::new(move |route: &str| {
                    let (before_query, _) = route.split_once('?').unwrap_or((route, ""));
                    let mut path = static_dir.clone();
                    for segment in before_query.split('/') {
                        path.push(segment);
                    }
                    path
                })
            }),
            invalidate_after,
        }
    }

    pub fn put(
        &mut self,
        route: String,
        timestamp: DateTime<Utc>,
        data: Vec<u8>,
    ) -> Result<(), IncrementalRendererError> {
        use std::io::Write;
        let file_path = self.route_as_path(&route, timestamp);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = std::fs::File::create(file_path)?;
        let mut file = std::io::BufWriter::new(file);
        file.write_all(&data)?;
        Ok(())
    }

    pub fn clear(&mut self) {
        // clear the static directory
        let _ = std::fs::remove_dir_all(&self.static_dir);
    }

    pub fn invalidate(&mut self, route: &str) {
        let file_path = self.find_file(route).unwrap().full_path;
        if let Err(err) = std::fs::remove_file(file_path) {
            tracing::error!("Failed to remove file: {}", err);
        }
    }

    pub fn get(
        &self,
        route: &str,
    ) -> Result<Option<(RenderFreshness, Vec<u8>)>, IncrementalRendererError> {
        if let Some(file_path) = self.find_file(route) {
            if let Some(freshness) = file_path.freshness(self.invalidate_after) {
                if let Ok(file) = std::fs::File::open(file_path.full_path) {
                    let mut file = std::io::BufReader::new(file);
                    let mut cache_hit = Vec::new();
                    std::io::copy(&mut file, &mut cache_hit)?;
                    tracing::trace!("file cache hit {:?}", route);
                    return Ok(Some((freshness, cache_hit)));
                }
            }
        }

        Ok(None)
    }

    fn find_file(&self, route: &str) -> Option<ValidCachedPath> {
        let mut file_path = (self.map_path)(route);
        if let Some(deadline) = self.invalidate_after {
            // find the first file that matches the route and is a html file
            file_path.push("index");
            if let Ok(dir) = std::fs::read_dir(file_path) {
                for entry in dir.flatten() {
                    if let Some(cached_path) = ValidCachedPath::try_from_path(entry.path()) {
                        if let Ok(elapsed) = cached_path.timestamp.elapsed() {
                            if elapsed < deadline {
                                // The timestamp is valid, return the file
                                return Some(cached_path);
                            }
                        }
                        // if the timestamp is invalid or passed, delete the file
                        if let Err(err) = std::fs::remove_file(entry.path()) {
                            tracing::error!("Failed to remove file: {}", err);
                        }
                    }
                }
                None
            } else {
                None
            }
        } else {
            file_path.push("index.html");
            file_path.exists().then_some({
                ValidCachedPath {
                    full_path: file_path,
                    timestamp: SystemTime::now(),
                }
            })
        }
    }

    fn route_as_path(&self, route: &str, timestamp: DateTime<Utc>) -> PathBuf {
        let mut file_path = (self.map_path)(route);
        if self.track_timestamps() {
            file_path.push("index");
            file_path.push(timestamp_to_string(timestamp));
        } else {
            file_path.push("index");
        }
        file_path.set_extension("html");
        file_path
    }

    fn track_timestamps(&self) -> bool {
        self.invalidate_after.is_some()
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

    pub fn freshness(&self, max_age: Option<std::time::Duration>) -> Option<RenderFreshness> {
        let age = self.timestamp.elapsed().ok()?.as_secs();
        let max_age = max_age.map(|max_age| max_age.as_secs());
        Some(RenderFreshness::new(age, max_age?, self.timestamp.into()))
    }
}

fn decode_timestamp(timestamp: &str) -> Option<std::time::SystemTime> {
    let timestamp = u64::from_str_radix(timestamp, 16).ok()?;
    Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp))
}

pub(crate) fn timestamp_to_string(timestamp: DateTime<Utc>) -> String {
    let timestamp = timestamp
        .signed_duration_since(DateTime::<Utc>::from(std::time::UNIX_EPOCH))
        .num_seconds();
    format!("{:x}", timestamp)
}
