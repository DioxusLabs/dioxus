use anyhow::anyhow;
use clap::Parser;
use dioxus_cli::{plugin::PluginManager, *};
use Commands::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    set_up_logging();

    let dioxus_config = DioxusConfig::load()
        .map_err(|e| anyhow!("Failed to load `dioxus.toml` because: {e}"))?
        .unwrap_or_else(|| {
            log::warn!("Your `dioxus.toml` could not be found. Using the default config. To set up this crate with dioxus, use `dioxus init`.");
            DioxusConfig::default()
        });

    PluginManager::init(dioxus_config.plugin)
        .map_err(|e| anyhow!("🚫 Plugin system initialization failed: {e}"))?;

    match args.action {
        Translate(opts) => opts
            .translate()
            .map_err(|e| anyhow!("🚫 Translation of HTML into RSX failed: {}", e)),

        Build(opts) => opts
            .build()
            .map_err(|e| anyhow!("🚫 Building project failed: {}", e)),

        Clean(opts) => opts
            .clean()
            .map_err(|e| anyhow!("🚫 Cleaning project failed: {}", e)),

        Serve(opts) => opts
            .serve()
            .await
            .map_err(|e| anyhow!("🚫 Serving project failed: {}", e)),

        Create(opts) => opts
            .create()
            .map_err(|e| anyhow!("🚫 Creating new project failed: {}", e)),

        Config(opts) => opts
            .config()
            .map_err(|e| anyhow!("🚫 Configuring new project failed: {}", e)),

        Plugin(opts) => opts
            .plugin()
            .await
            .map_err(|e| anyhow!("🚫 Error with plugin: {}", e)),

        Autoformat(opts) => opts
            .autoformat()
            .map_err(|e| anyhow!("🚫 Error autoformatting RSX: {}", e)),

        Version(opt) => {
            let version = opt.version();
            println!("{}", version);

            Ok(())
        }
    }
}
