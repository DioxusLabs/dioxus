/// Create an file asset from the local path or url to the file
///
/// > **Note**: This will do nothing outside of the `asset!` macro
///
/// The file builder collects an arbitrary file. Relative paths are resolved relative to the package root
/// ```rust
/// const _: &str = manganis::asset!("/assets/asset.txt");
/// ```
/// Or you can use URLs to read the asset at build time from a remote location
/// ```rust
/// const _: &str = manganis::asset!("https://rustacean.net/assets/rustacean-flat-happy.png");
/// ```
#[allow(unused)]
pub const fn file(path: &'static str) -> &'static str {
    path
}
