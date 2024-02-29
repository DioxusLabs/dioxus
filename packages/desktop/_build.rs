fn check_gnu() {
    // WARN about wry support on windows gnu targets. GNU windows targets don't work well in wry currently
    if std::env::var("CARGO_CFG_WINDOWS").is_ok()
        && std::env::var("CARGO_CFG_TARGET_ENV").unwrap() == "gnu"
        && !cfg!(feature = "gnu")
    {
        println!("cargo:warning=GNU windows targets have some limitations within Wry. Using the MSVC windows toolchain is recommended. If you would like to use continue using GNU, you can read https://github.com/wravery/webview2-rs#cross-compilation and disable this warning by adding the gnu feature to dioxus-desktop in your Cargo.toml")
    }
}

fn main() {
    check_gnu();
}
