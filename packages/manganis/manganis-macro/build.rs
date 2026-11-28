fn main() {
    println!("cargo:rerun-if-env-changed=MANGANIS_EMBED_ASSETS");
    let value = std::env::var("MANGANIS_EMBED_ASSETS").unwrap_or_default();
    let value = match value.as_str() {
        "0" => "0",
        "false" => "false",
        _ => "",
    };
    println!("cargo:rustc-env=MANGANIS_EMBED_ASSETS_RESOLVED={value}");
}
