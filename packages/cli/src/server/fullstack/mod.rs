use crate::{
    cfg::{ConfigOptsBuild, ConfigOptsServe},
    CrateConfig, Result, WebAssetConfigDropGuard,
};

use super::{desktop, Platform};

pub async fn startup(config: CrateConfig, serve: &ConfigOptsServe) -> Result<()> {
    desktop::startup_with_platform::<FullstackPlatform>(config, serve).await
}

struct FullstackPlatform {
    serve: ConfigOptsServe,
    desktop: desktop::DesktopPlatform,
    _config: WebAssetConfigDropGuard,
}

impl Platform for FullstackPlatform {
    fn start(config: &CrateConfig, serve: &ConfigOptsServe) -> Result<Self>
    where
        Self: Sized,
    {
        {
            build_web(serve.clone())?;
        }

        let mut desktop_config = config.clone();
        let desktop_feature = serve.server_feature.clone();
        let features = &mut desktop_config.features;
        match features {
            Some(features) => {
                features.push(desktop_feature);
            }
            None => desktop_config.features = Some(vec![desktop_feature]),
        };
        let config = WebAssetConfigDropGuard::new();
        let desktop = desktop::DesktopPlatform::start(&desktop_config, serve)?;

        Ok(Self {
            desktop,
            serve: serve.clone(),
            _config: config,
        })
    }

    fn rebuild(&mut self, crate_config: &CrateConfig) -> Result<crate::BuildResult> {
        build_web(self.serve.clone())?;
        {
            let mut desktop_config = crate_config.clone();
            let desktop_feature = self.serve.server_feature.clone();
            let features = &mut desktop_config.features;
            match features {
                Some(features) => {
                    features.push(desktop_feature);
                }
                None => desktop_config.features = Some(vec![desktop_feature]),
            };
            let _gaurd = FullstackServerEnvGuard::new(self.serve.debug);
            self.desktop.rebuild(&desktop_config)
        }
    }
}

fn build_web(serve: ConfigOptsServe) -> Result<()> {
    let mut web_config: ConfigOptsBuild = serve.into();
    let web_feature = web_config.client_feature.clone();
    let features = &mut web_config.features;
    match features {
        Some(features) => {
            features.push(web_feature);
        }
        None => web_config.features = Some(vec![web_feature]),
    };
    web_config.platform = Some(crate::cfg::Platform::Web);

    let _gaurd = FullstackWebEnvGuard::new(web_config.debug);
    crate::cli::build::Build { build: web_config }.build(None)
}

// Debug mode web builds have a very large size by default. If debug mode is not enabled, we strip some of the debug info by default
// This reduces a hello world from ~40MB to ~2MB
pub(crate) struct FullstackWebEnvGuard {
    old_rustflags: Option<String>,
}

impl FullstackWebEnvGuard {
    pub fn new(debug_mode: bool) -> Self {
        Self {
            old_rustflags: (!debug_mode).then(|| {
                let old_rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();

                std::env::set_var(
                    "RUSTFLAGS",
                    format!("{old_rustflags} -C debuginfo=none -C strip=debuginfo"),
                );
                old_rustflags
            }),
        }
    }
}

impl Drop for FullstackWebEnvGuard {
    fn drop(&mut self) {
        if let Some(old_rustflags) = self.old_rustflags.take() {
            std::env::set_var("RUSTFLAGS", old_rustflags);
        }
    }
}

// Debug mode web builds have a very large size by default. If debug mode is not enabled, we strip some of the debug info by default
// This reduces a hello world from ~40MB to ~2MB
pub(crate) struct FullstackServerEnvGuard {
    old_rustflags: Option<String>,
}

impl FullstackServerEnvGuard {
    pub fn new(debug_mode: bool) -> Self {
        Self {
            old_rustflags: (!debug_mode).then(|| {
                let old_rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();

                std::env::set_var("RUSTFLAGS", format!("{old_rustflags} -C opt-level=2"));
                old_rustflags
            }),
        }
    }
}

impl Drop for FullstackServerEnvGuard {
    fn drop(&mut self) {
        if let Some(old_rustflags) = self.old_rustflags.take() {
            std::env::set_var("RUSTFLAGS", old_rustflags);
        }
    }
}
