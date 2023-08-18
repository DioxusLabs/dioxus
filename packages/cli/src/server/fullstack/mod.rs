use crate::{
    cfg::{ConfigOptsBuild, ConfigOptsServe},
    CrateConfig, Result,
};

use super::{desktop, Platform};

pub async fn startup(config: CrateConfig, serve: &ConfigOptsServe) -> Result<()> {
    desktop::startup_with_platform::<FullstackPlatform>(config, serve).await
}

struct FullstackPlatform {
    serve: ConfigOptsServe,
    desktop: desktop::DesktopPlatform,
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
        let desktop = desktop::DesktopPlatform::start(&desktop_config, serve)?;

        Ok(Self {
            desktop,
            serve: serve.clone(),
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
    crate::cli::build::Build { build: web_config }.build(None)
}
