/// This is basically a compile-time version of ResourceAsset
/// A struct that contains the relative and absolute paths of an asset
#[derive(Debug, PartialEq, PartialOrd, Clone, Hash)]
pub struct Asset {
    /// The input URI given to the macro
    pub input: &'static str,

    /// The sourcefile of the asset
    pub source_file: &'static str,

    ///
    pub local: &'static str,

    ///
    pub bundled: &'static str,
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.resolve().fmt(f)
    }
}

impl From<Asset> for String {
    fn from(asset: Asset) -> Self {
        asset.resolve()
    }
}

impl From<Asset> for Option<String> {
    fn from(asset: Asset) -> Self {
        Some(asset.resolve())
    }
}

impl Asset {
    /// Resolve the asset against the bundle
    pub fn resolve(&self) -> String {
        // A fallback for non-bundled apps with no support for manganis
        //
        // Necessary to get `cargo run` to work when folks use `cargo run --example demo` on the main
        // dioxus repo.
        //
        // We could also say, suggest that they install `dioxus-cli` and use that instead.
        if local_fallback() {
            return self.bundled.to_string();
        }

        // the rest of the platforms are bundled, so we need to resolve the asset against the bundle

        // for web, we just do the basepath thing
        #[cfg(target_arch = "wasm32")]
        {
            return format!("/{}", self.bundled);
        }

        // On mac do a bundle lookup
        #[cfg(target_os = "macos")]
        {
            let bundle = core_foundation::bundle::CFBundle::main_bundle();
            let bundle_path = bundle.path().unwrap();
            let resources_path = bundle.resources_path().unwrap();
            let absolute_resources_root = bundle_path.join(resources_path);
            return dunce::canonicalize(absolute_resources_root)
                .ok()
                .unwrap()
                .display()
                .to_string();
        }

        // // on ios do a bundle lookup
        // #[cfg(target_os = "ios")]
        // {
        //     let bundle = core_foundation::bundle::CFBundle::main_bundle();
        //     let bundle_path = bundle.path().unwrap();
        //     let resources_path = bundle.resources_path().unwrap();
        //     let absolute_resources_root = bundle_path.join(resources_path);
        //     return dunce::canonicalize(absolute_resources_root)
        //         .ok()
        //         .unwrap()
        //         .display()
        //         .to_string();
        // }

        // on android do a bundle lookup

        // on windows,

        todo!()
    }

    fn name(&self) -> String {
        if BUNDLED {
            self.input.to_string()
        } else {
            self.local.to_string()
        }
    }
}

static BUNDLED: bool = false;
// static BUNDLED: bool = option_env!("MG_BUNDLED").is_some();

/// Returns whether the app should use the local fallback or not
///
/// A `cargo run` will not be bundled but the asset will be resolved against the filesystem through
/// dependencies.
pub fn local_fallback() -> bool {
    // If we're bundled, manganis is active
    if BUNDLED {
        return false;
    }

    // Otherwise, check if the MG_RUNTIME env var is set
    // this prevents us from thrashing the cache when running `cargo run`
    static USE_FALLBACK: once_cell::sync::OnceCell<bool> = once_cell::sync::OnceCell::new();
    *USE_FALLBACK.get_or_init(|| {
        // If the env var is present, we use the bundled path
        if std::env::var("MG_RUNTIME").is_ok() {
            return false;
        }

        // on wasm, there's no env vars... but the app is not bundled
        // for now we just assume you're using manganis in a wasm app
        if cfg!(target_arch = "wasm32") {
            return false;
        }

        // No env var, not wasm, not bundled, so we're not using manganis
        true
    })
}
