//! Object file cache for workspace hotpatching.
//!
//! Maintains the latest `.rcgu.o` files for each crate in the cumulative `modified_crates` set.
//! Used for accumulated relinking: combining objects from all modified crates into a single
//! patch dylib.
//!
//! Objects are stored on disk under `session_cache_dir/object_cache/{crate_name}/` so they
//! persist across patches without holding file contents in memory. The session cache dir
//! lives in `/tmp/` and is cleaned up by the OS.
//!
//! - **Dep crates:** objects are extracted from their rlib in `target/deps/`.
//! - **Tip crate:** objects are copied from linker arg paths since incremental compilation
//!   overwrites them in place.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Cache of compiled object files on disk, keyed by crate name.
///
/// After each compilation, the cache is updated for the compiled crate by extracting
/// objects to `dir/{crate_name}/`. On relink, paths from all crates in `modified_crates`
/// are passed directly to the linker — no intermediate copy needed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectCache {
    /// Root directory: `session_cache_dir/object_cache/`
    dir: PathBuf,

    /// crate_name -> object file paths on disk
    objects: HashMap<String, Vec<PathBuf>>,
}

impl ObjectCache {
    pub fn new(session_cache_dir: &Path) -> Self {
        let dir = session_cache_dir.join("object_cache");
        Self {
            dir,
            objects: HashMap::new(),
        }
    }

    /// Extract `.rcgu.o` files from an rlib archive and write them to
    /// `dir/{crate_name}/`. Replaces any previously cached objects for this crate.
    pub fn cache_from_rlib(&mut self, crate_name: &str, rlib_path: &Path) -> anyhow::Result<()> {
        let crate_dir = self.dir.join(crate_name);
        // Clear previous objects for this crate
        if crate_dir.exists() {
            std::fs::remove_dir_all(&crate_dir)?;
        }
        std::fs::create_dir_all(&crate_dir)?;

        let rlib_contents = std::fs::read(rlib_path)?;
        let mut reader = ar::Archive::new(std::io::Cursor::new(rlib_contents));
        let mut paths = Vec::new();

        while let Some(Ok(mut entry)) = reader.next_entry() {
            let name = std::str::from_utf8(entry.header().identifier())
                .unwrap_or_default()
                .to_string();

            // Skip rmeta and empty entries
            if name.ends_with(".rmeta") || entry.header().size() == 0 {
                continue;
            }

            // Only keep .rcgu.o object files
            if !name.ends_with(".rcgu.o") && !name.ends_with(".o") {
                continue;
            }

            let mut data = Vec::with_capacity(entry.header().size() as usize);
            entry.read_to_end(&mut data)?;

            let obj_path = crate_dir.join(&name);
            std::fs::write(&obj_path, &data)?;
            paths.push(obj_path);
        }

        self.objects.insert(crate_name.to_string(), paths);
        Ok(())
    }

    /// Cache tip crate objects by copying them from their filesystem paths.
    ///
    /// Tip crate `.rcgu.o` files get overwritten on recompilation, so we copy
    /// them into our cache directory for stable access.
    pub fn cache_from_paths(
        &mut self,
        crate_name: &str,
        object_paths: &[impl AsRef<Path>],
    ) -> anyhow::Result<()> {
        let crate_dir = self.dir.join(crate_name);
        if crate_dir.exists() {
            std::fs::remove_dir_all(&crate_dir)?;
        }
        std::fs::create_dir_all(&crate_dir)?;

        let mut paths = Vec::with_capacity(object_paths.len());

        for path in object_paths {
            let path = path.as_ref();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let dest = crate_dir.join(name);
            std::fs::copy(path, &dest)?;
            paths.push(dest);
        }

        self.objects.insert(crate_name.to_string(), paths);
        Ok(())
    }

    /// Get cached object file paths for a crate.
    pub fn get(&self, crate_name: &str) -> Option<&Vec<PathBuf>> {
        self.objects.get(crate_name)
    }
}
