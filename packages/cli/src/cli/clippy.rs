use super::*;
use crate::{AppBuilder, BuildId, BuildMode};

/// Run clippy on the project through the dx build pipeline.
///
/// Uses dx's cargo configuration (target, features, profile) so the lints see exactly the code that
/// `dx build` compiles, and injects Dioxus-aware clippy.toml entries per target.
#[derive(Clone, Debug, Parser)]
pub(crate) struct Clippy {
    #[clap(flatten)]
    pub(crate) build_args: CommandWithPlatformOverrides<BuildArgs>,

    /// Arguments passed to clippy, e.g. `dx clippy -- -D warnings`
    #[clap(last = true)]
    pub(crate) clippy_args: Vec<String>,
}

impl Clippy {
    pub(crate) async fn clippy(self) -> Result<StructuredOutput> {
        let BuildTargets {
            mut client,
            mut server,
        } = self.build_args.into_targets().await?;

        client.clippy_args = self.clippy_args.clone();
        if let Some(server) = server.as_mut() {
            server.clippy_args = self.clippy_args.clone();
        }

        let mode = BuildMode::Check { clippy: true };

        AppBuilder::started(&client, mode.clone(), BuildId::PRIMARY)?
            .finish_build()
            .await?;
        tracing::info!(
            "Clippy finished for {} [{}]",
            client.main_target,
            client.triple
        );

        if let Some(server) = server {
            AppBuilder::started(&server, mode, BuildId::SECONDARY)?
                .finish_build()
                .await?;
            tracing::info!(
                "Clippy finished for {} [{}]",
                server.main_target,
                server.triple
            );
        }

        Ok(StructuredOutput::Success)
    }
}
