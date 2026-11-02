#![allow(non_snake_case)]

use chrono::{DateTime, Utc};

use super::{IncrementalRendererError, RenderFreshness};
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

pub(crate) type PathMapFn = Arc<dyn Fn(&str) -> PathBuf + Send + Sync>;

pub(crate) struct FileSystemCache {
    static_dir: PathBuf,
    map_path: Option<PathMapFn>,
    invalidate_after: Option<std::time::Duration>,
}

impl FileSystemCache {
    pub fn new(
        static_dir: PathBuf,
        map_path: Option<PathMapFn>,
        invalidate_after: Option<std::time::Duration>,
    ) -> Self {
        Self {
            static_dir,
            map_path,
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
        let Some(file_path) = self.route_as_path(&route, timestamp) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("route {route:?} cannot be mapped to a path inside the static directory"),
            )
            .into());
        };
        if let Some(parent) = file_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(file_path)?;
        let mut file = std::io::BufWriter::new(file);
        file.write_all(&data)?;
        Ok(())
    }

    pub fn clear(&mut self) {
        // clear the static directory of index.html files contained within folders
        for entry in std::fs::read_dir(&self.static_dir)
            .into_iter()
            .flatten()
            .flatten()
        {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                for entry in walkdir::WalkDir::new(entry.path()).into_iter().flatten() {
                    if entry.file_type().is_file()
                        && let Some(fnmae) = entry.file_name().to_str()
                        && fnmae.ends_with(".html")
                        && let Err(err) = std::fs::remove_file(entry.path())
                    {
                        tracing::error!("Failed to remove file: {}", err);
                    }
                }
            }
        }
    }

    pub fn invalidate(&mut self, route: &str) {
        let Some(cached) = self.find_file(route) else {
            return;
        };
        if let Err(err) = std::fs::remove_file(cached.full_path) {
            tracing::error!("Failed to remove file: {}", err);
        }
    }

    pub fn get(
        &self,
        route: &str,
    ) -> Result<Option<(RenderFreshness, Vec<u8>)>, IncrementalRendererError> {
        if let Some(file_path) = self.find_file(route)
            && let Some(freshness) = file_path.freshness(self.invalidate_after)
            && let Ok(file) = std::fs::File::open(file_path.full_path)
        {
            let mut file = std::io::BufReader::new(file);
            let mut cache_hit = Vec::new();
            std::io::copy(&mut file, &mut cache_hit)?;
            tracing::trace!("file cache hit {:?}", route);
            return Ok(Some((freshness, cache_hit)));
        }

        Ok(None)
    }

    /// The directory the cached `index.html` for `route` lives in, or `None` if the route can't be cached.
    fn route_dir(&self, route: &str) -> Option<PathBuf> {
        match &self.map_path {
            Some(map_path) => Some(map_path(route)),
            None => default_route_dir(&self.static_dir, route),
        }
    }

    fn find_file(&self, route: &str) -> Option<ValidCachedPath> {
        let mut file_path = self.route_dir(route)?;
        if let Some(deadline) = self.invalidate_after {
            // find the first file that matches the route and is a html file
            file_path.push("index");
            if let Ok(dir) = std::fs::read_dir(file_path) {
                for entry in dir.flatten() {
                    if let Some(cached_path) = ValidCachedPath::try_from_path(entry.path()) {
                        if let Ok(elapsed) = cached_path.timestamp.elapsed()
                            && elapsed < deadline
                        {
                            // The timestamp is valid, return the file
                            return Some(cached_path);
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

    fn route_as_path(&self, route: &str, timestamp: DateTime<Utc>) -> Option<PathBuf> {
        let mut file_path = self.route_dir(route)?;
        if self.track_timestamps() {
            file_path.push("index");
            file_path.push(timestamp_to_string(timestamp));
        } else {
            file_path.push("index");
        }
        file_path.set_extension("html");
        Some(file_path)
    }

    fn track_timestamps(&self) -> bool {
        self.invalidate_after.is_some()
    }
}

/// Map a route to a directory inside `static_dir`. Returns `None` if any segment of the route
/// could resolve outside of `static_dir` (`.`, `..`, a path separator or a drive prefix).
fn default_route_dir(static_dir: &Path, route: &str) -> Option<PathBuf> {
    let (before_query, _) = route.split_once('?').unwrap_or((route, ""));
    let mut path = static_dir.to_path_buf();
    for segment in before_query
        .split('/')
        .filter(|segment| !segment.is_empty())
    {
        let mut components = Path::new(segment).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(name)), None) if name == segment => path.push(name),
            _ => return None,
        }
    }
    Some(path)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_route_dir_stays_inside_static_dir() {
        let static_dir = Path::new("/srv/static");
        assert_eq!(
            default_route_dir(static_dir, "/"),
            Some(static_dir.to_path_buf())
        );
        assert_eq!(
            default_route_dir(static_dir, "/blog/1/?q=../x"),
            Some(static_dir.join("blog").join("1"))
        );
        assert_eq!(
            default_route_dir(static_dir, "/%2e%2e/x"),
            Some(static_dir.join("%2e%2e").join("x"))
        );
        for route in [
            "/../victim",
            "/a/../../victim",
            "../victim",
            "/./a",
            "/a/..",
        ] {
            assert_eq!(default_route_dir(static_dir, route), None, "{route}");
        }
    }

    #[test]
    fn put_rejects_routes_outside_static_dir() {
        let root = std::env::temp_dir().join(format!("dioxus-isr-{}", std::process::id()));
        let static_dir = root.join("static");
        std::fs::create_dir_all(&static_dir).unwrap();
        let mut cache = FileSystemCache::new(static_dir.clone(), None, None);

        assert!(
            cache
                .put("/../victim".to_string(), Utc::now(), b"pwned".to_vec())
                .is_err()
        );
        assert!(!root.join("victim").exists());
        assert!(cache.get("/../victim").unwrap().is_none());
        cache.invalidate("/../victim");

        cache
            .put("/blog".to_string(), Utc::now(), b"ok".to_vec())
            .unwrap();
        assert!(static_dir.join("blog").join("index.html").exists());

        std::fs::remove_dir_all(root).unwrap();
    }
}
