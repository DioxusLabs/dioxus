fn main() {
    // When the `embed` feature is enabled, rust-embed's derive macro needs DIOXUS_EMBED_DIR
    // to point to a folder. The CLI sets this to the client's public output directory during
    // `dx build --embed`. For regular development (clippy, cargo check --all-features, IDE),
    // provide an empty fallback so the derive compiles with zero embedded assets.
    if cfg!(feature = "embed") && std::env::var("DIOXUS_EMBED_DIR").is_err() {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let fallback = format!("{out_dir}/empty_embed");
        std::fs::create_dir_all(&fallback).unwrap();
        println!("cargo:rustc-env=DIOXUS_EMBED_DIR={fallback}");
    }
}
