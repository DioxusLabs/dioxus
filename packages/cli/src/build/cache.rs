//! Object file cache for workspace hotpatching.
//!
//! Maintains the latest `.rcgu.o` files for each crate in the cumulative `modified_crates` set.
//! Used for assembly diffing (comparing old vs new objects) and accumulated relinking
//! (combining objects from all modified crates into a single patch dylib).
//!
//! - **Dep crates:** objects are extracted from their rlib in `target/deps/`.
//! - **Tip crate:** objects come from linker arg interception and must be copied since
//!   incremental compilation overwrites them.

use bytes::Bytes;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// A single compiled object file from a crate.
#[derive(Clone, Debug, PartialEq)]
pub struct CachedObject {
    /// Original filename (e.g., "foo-abc123.rcgu.o")
    pub name: String,
    /// Object file contents
    pub data: Bytes,
}

/// Cache of compiled object files, keyed by crate name.
///
/// After each compilation, the cache is updated for the compiled crate.
/// On relink, objects from all crates in `modified_crates` are combined.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectCache {
    /// crate_name -> compiled object files from that crate
    pub objects: HashMap<String, Vec<CachedObject>>,
}

impl ObjectCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract `.rcgu.o` files from an rlib archive and cache them under the given crate name.
    ///
    /// This is used for dependency crates whose rlib is the canonical source of their objects.
    /// Skips `.rmeta` files and empty entries.
    pub fn cache_from_rlib(&mut self, crate_name: &str, rlib_path: &Path) -> anyhow::Result<()> {
        let rlib_contents = std::fs::read(rlib_path)?;
        let mut reader = ar::Archive::new(std::io::Cursor::new(rlib_contents));
        let mut objects = Vec::new();

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
            objects.push(CachedObject {
                name,
                data: Bytes::from(data),
            });
        }

        self.objects.insert(crate_name.to_string(), objects);
        Ok(())
    }

    /// Cache tip crate objects from their filesystem paths (extracted from linker args).
    ///
    /// Tip crate `.rcgu.o` files live on disk in the target directory but get overwritten
    /// on recompilation, so we read and cache their contents.
    pub fn cache_from_paths(
        &mut self,
        crate_name: &str,
        object_paths: &[impl AsRef<Path>],
    ) -> anyhow::Result<()> {
        let mut objects = Vec::with_capacity(object_paths.len());

        for path in object_paths {
            let path = path.as_ref();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let data = std::fs::read(path)?;
            objects.push(CachedObject {
                name,
                data: Bytes::from(data),
            });
        }

        self.objects.insert(crate_name.to_string(), objects);
        Ok(())
    }

    /// Get cached objects for a crate.
    pub fn get(&self, crate_name: &str) -> Option<&Vec<CachedObject>> {
        self.objects.get(crate_name)
    }

}
