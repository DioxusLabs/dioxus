use crate::{
    platform_override::CommandWithPlatformOverrides, AppBuilder, BuildArgs, BuildMode,
    HotpatchModuleCache, Result, StructuredOutput,
};
use anyhow::Context;
use clap::Parser;
use dioxus_dx_wire_format::StructuredBuildArtifacts;
use std::sync::Arc;

const HELP_HEADING: &str = "Hotpatching a binary";

/// Patches a single binary, but takes the same arguments as a `cargo build`.
///
/// By default, patches the client, but you can set patch_server to true to patch the server instead.
#[derive(Clone, Debug, Parser)]
pub struct HotpatchTip {
    /// Should we patch the server or the client? False = client, True = server
    #[clap(long, num_args = 0..=1, default_missing_value="true", help_heading = HELP_HEADING)]
    pub patch_server: Option<bool>,

    /// The serialized output from the `dx build --fat-binary` command
    #[clap(long, help_heading = HELP_HEADING)]
    pub serialized_artifacts: String,

    /// The ASLR reference of the running app being patched. Used to generate sensible offsets for patched code.
    #[clap(long, help_heading = HELP_HEADING)]
    pub aslr_reference: u64,

    #[clap(flatten)]
    pub build_args: CommandWithPlatformOverrides<BuildArgs>,
}

impl HotpatchTip {
    pub async fn run(self) -> Result<StructuredOutput> {
        let targets = self.build_args.into_targets().await?;

        // Select which target to patch
        let request = if self.patch_server.unwrap_or_default() {
            targets.server.as_ref().context("No server to patch!")?
        } else {
            &targets.client
        };

        let structured_build_artifacts =
            serde_json::from_str::<StructuredBuildArtifacts>(&self.serialized_artifacts)
                .context("Failed to parse structured build artifacts")?;

        let StructuredBuildArtifacts {
            exe,
            rustc_args,
            rustc_envs,
            link_args,
            ..
        } = structured_build_artifacts;

        // todo: loading this cache over and over defeats the purpose of a cache
        //       consider a shared-mem approach or a binary serializer? something like arrow / parquet / bincode?
        let cache = Arc::new(HotpatchModuleCache::new(&exe, &request.triple)?);

        let mode = BuildMode::Thin {
            rustc_args: crate::RustcArgs {
                args: rustc_args,
                envs: rustc_envs,
                link_args,
            },
            changed_files: vec![],
            aslr_reference: self.aslr_reference,
            cache: cache.clone(),
        };

        let artifacts = AppBuilder::started(request, mode)?.finish_build().await?;
        let patch_exe = request.patch_exe(artifacts.time_start);

        Ok(StructuredOutput::Hotpatch {
            jump_table: crate::build::create_jump_table(&patch_exe, &request.triple, &cache)?,
            artifacts: artifacts.into_structured_output(),
        })
    }
}
