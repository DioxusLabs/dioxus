// The file has been placed there by the build script.
include!(concat!(env!("OUT_DIR"), "/built.rs"));

pub(crate) fn version() -> String {
    format!(
        "{} ({})",
        PKG_VERSION,
        GIT_COMMIT_HASH_SHORT.unwrap_or("was built without git repository")
    )
}
