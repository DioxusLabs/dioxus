use crate::{
    platform_override::CommandWithPlatformOverrides, AppBuilder, BuildArgs, BuildId, BuildMode,
    HotpatchModuleCache, Result, StructuredOutput,
};
use anyhow::Context;
use clap::Parser;
use dioxus_dx_wire_format::StructuredBuildArtifacts;
use std::io::Read;
use std::sync::Arc;

const HELP_HEADING: &str = "Hotpatching a binary";

/// Patches a single binary, but takes the same arguments as a `cargo build` and the serialized output from `dx build --fat-binary` as input.
///
/// This is intended to be used with something like `dx build --fat-binary >> output.json` and then
/// `cat output.json | dx hotpatch --aslr-reference 0x12345678` to produce a hotpatched binary.
///
/// By default, patches the client, but you can set patch_server to true to patch the server instead.
#[derive(Clone, Debug, Parser)]
pub struct HotpatchTip {
    /// Should we patch the server or the client? False = client, True = server
    #[clap(long, num_args = 0..=1, default_missing_value="true", help_heading = HELP_HEADING)]
    pub patch_server: Option<bool>,

    /// The ASLR reference of the running app being patched. Used to generate sensible offsets for patched code.
    #[clap(long, help_heading = HELP_HEADING)]
    pub aslr_reference: u64,

    #[clap(flatten)]
    pub build_args: CommandWithPlatformOverrides<BuildArgs>,
}

impl HotpatchTip {
    pub async fn run(self) -> Result<StructuredOutput> {
        let targets = self.build_args.into_targets().await?;

        let patch_server = self.patch_server.unwrap_or(false);

        let build_id = if patch_server {
            BuildId::SECONDARY
        } else {
            BuildId::PRIMARY
        };

        // Select which target to patch
        let request = if patch_server {
            targets.server.as_ref().context("No server to patch!")?
        } else {
            &targets.client
        };

        let mut serialized_artifacts = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut serialized_artifacts)
            .context("Failed to read serialized build artifacts from stdin")?;
        let structured_build_artifacts =
            serde_json::from_str::<StructuredBuildArtifacts>(&serialized_artifacts)
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

        let tip_crate_name = request.main_target.replace('-', "_");
        let mut workspace_rustc_args = std::collections::HashMap::new();
        workspace_rustc_args.insert(
            tip_crate_name,
            crate::RustcArgs {
                args: rustc_args,
                envs: rustc_envs,
                link_args,
            },
        );
        let mode = BuildMode::Thin {
            workspace_rustc_args,
            changed_files: vec![],
            changed_crates: vec![],
            modified_crates: std::collections::HashSet::new(),
            aslr_reference: self.aslr_reference,
            cache: cache.clone(),
            object_cache: crate::ObjectCache::new(&request.session_cache_dir()),
        };

        let artifacts = AppBuilder::started(request, mode, build_id)?
            .finish_build()
            .await?;
        let patch_exe = request.patch_exe(artifacts.time_start);

        Ok(StructuredOutput::Hotpatch {
            jump_table: request.create_jump_table(&patch_exe, &cache)?,
            artifacts: artifacts.into_structured_output(),
        })
    }
}
