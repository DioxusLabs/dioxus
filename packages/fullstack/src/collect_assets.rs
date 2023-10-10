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
        // The CLI will copy assets to the current working directory
        if std::env::var_os("DIOXUS_ACTIVE").is_some() {
            return;
        }
        use manganis_cli_support::AssetManifest;
        use manganis_cli_support::AssetManifestExt;
        use manganis_cli_support::Config;
        use std::path::PathBuf;
        let config = Config::current();
        let asset_location = config.assets_serve_location();
        let asset_location = PathBuf::from(asset_location);
        let _ = std::fs::remove_dir_all(&asset_location);

        println!("Finding assets... (Note: if you run a dioxus desktop application with the CLI. This process will be significantly faster.)");
        let manifest = AssetManifest::load();
        let has_assets = manifest
            .packages()
            .iter()
            .any(|package| !package.assets().is_empty());

        if has_assets {
            println!("Copying and optimizing assets...");
            manifest.copy_static_assets_to(&asset_location).unwrap();
            println!("Copied assets to {}", asset_location.display());
        } else {
            println!("No assets found");
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
