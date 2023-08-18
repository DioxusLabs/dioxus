pub fn copy_assets() {
    #[cfg(all(
        debug_assertions,
        any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )
    ))]
    {
        use manganis_cli_support::AssetManifest;
        use manganis_cli_support::AssetManifestExt;
        use manganis_cli_support::Config;
        use std::path::PathBuf;
        let config = Config::current();
        let asset_location = config.assets_serve_location();
        let asset_location = PathBuf::from(asset_location);
        let _ = std::fs::remove_dir_all(&asset_location);

        let manifest = AssetManifest::load();
        let has_assets = manifest
            .packages()
            .iter()
            .any(|package| !package.assets().is_empty());

        if has_assets {
            println!("Copying and optimizing assets...");
            manifest.copy_static_assets_to(&asset_location).unwrap();
            println!("Copied assets to {}", asset_location.display());
        }
    }
    #[cfg(not(all(
        debug_assertions,
        any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )
    )))]
    {
        println!(
            "Skipping assets in release mode. You compile assets with the dioxus-cli in release mode"
        );
    }
}
