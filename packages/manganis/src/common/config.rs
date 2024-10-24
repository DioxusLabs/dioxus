use std::path::PathBuf;

/// Get the base path for assets defined by the MG_BASEPATH environment variable
///
/// The basepath should always start and end with a `/`
///
/// If no basepath is set, the default is `/` which is the root of the assets folder.
pub fn base_path() -> PathBuf {
    "/".into()
    // match option_env!("MG_BASEPATH") {
    //     Some(path) => {
    //         let path = path.trim_end_matches('/').trim_start_matches('/');
    //         PathBuf::from(format!("/{path}/"))
    //     }
    //     None => "/".into(),
    // }
}

/// MG_BUNDLED is set to true when the application is bundled.
///
/// When running under a dev server, this is false to prevent thrashing of the cache since an ordinary
/// `cargo check` will not pass MG_BUNDLED.
pub const fn is_bundled() -> bool {
    false
    // option_env!("MG_BUNDLED").is_some()
}

/// The location of the manifest directory used to build this crate
pub fn manifest_dir() -> Option<PathBuf> {
    std::env::var("CARGO_MANIFEST_DIR").ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_path_works() {
        assert_eq!(base_path(), PathBuf::from("/"));
    }
}
