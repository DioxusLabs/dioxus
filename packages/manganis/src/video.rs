/// Create a video asset from the local path or url to the video
///
/// > **Note**: This will do nothing outside of the `mg!` macro
///
/// The video builder collects an arbitrary file. Relative paths are resolved relative to the package root
/// ```rust
/// const _: &str = manganis::mg!(video("/assets/video.mp4"));
/// ```
/// Or you can use URLs to read the asset at build time from a remote location
/// ```rust
/// const _: &str = manganis::mg!(video("https://private-user-images.githubusercontent.com/66571940/355646745-10781eef-de07-491d-aaa3-f75949b32190.mov?jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJnaXRodWIuY29tIiwiYXVkIjoicmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbSIsImtleSI6ImtleTUiLCJleHAiOjE3MjMxMzI5NTcsIm5iZiI6MTcyMzEzMjY1NywicGF0aCI6Ii82NjU3MTk0MC8zNTU2NDY3NDUtMTA3ODFlZWYtZGUwNy00OTFkLWFhYTMtZjc1OTQ5YjMyMTkwLm1vdj9YLUFtei1BbGdvcml0aG09QVdTNC1ITUFDLVNIQTI1NiZYLUFtei1DcmVkZW50aWFsPUFLSUFWQ09EWUxTQTUzUFFLNFpBJTJGMjAyNDA4MDglMkZ1cy1lYXN0LTElMkZzMyUyRmF3czRfcmVxdWVzdCZYLUFtei1EYXRlPTIwMjQwODA4VDE1NTczN1omWC1BbXotRXhwaXJlcz0zMDAmWC1BbXotU2lnbmF0dXJlPTVkODEwZjI4ODE2ZmM4MjE3MWQ2ZDk3MjQ0NjQxYmZlMDI2OTAyMzhjNGU4MzlkYTdmZWM1MjI4ZWQ5NDg3M2QmWC1BbXotU2lnbmVkSGVhZGVycz1ob3N0JmFjdG9yX2lkPTAma2V5X2lkPTAmcmVwb19pZD0wIn0.jlX5E6WGjZeqZind6UCRLFrJ9NHcsV8xXy-Ls30tKPQ"));
/// ```
#[allow(unused)]
pub const fn video(path: &'static str) -> &'static str {
    path
}
