use anyhow::anyhow;
use clap::Parser;
use dioxus_cli::*;

#[cfg(feature = "plugin")]
use dioxus_cli::plugin::PluginManager;

use Commands::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    set_up_logging();

    let _dioxus_config = DioxusConfig::load(args.bin.clone())
        .map_err(|e| anyhow!("Failed to load Dioxus config because: {e}"))?
        .unwrap_or_else(|| {
            log::warn!("You appear to be creating a Dioxus project from scratch; we will use the default config");
            DioxusConfig::default()
        });

    #[cfg(feature = "plugin")]
    PluginManager::init(_dioxus_config.plugin)
        .map_err(|e| anyhow!("🚫 Plugin system initialization failed: {e}"))?;

    match args.action {
        Translate(opts) => opts
            .translate()
            .map_err(|e| anyhow!("🚫 Translation of HTML into RSX failed: {}", e)),

        Build(opts) => opts
            .build(args.bin.clone())
            .map_err(|e| anyhow!("🚫 Building project failed: {}", e)),

        Clean(opts) => opts
            .clean(args.bin.clone())
            .map_err(|e| anyhow!("🚫 Cleaning project failed: {}", e)),

        Serve(opts) => opts
            .serve(args.bin.clone())
            .await
            .map_err(|e| anyhow!("🚫 Serving project failed: {}", e)),

        Create(opts) => opts
            .create()
            .map_err(|e| anyhow!("🚫 Creating new project failed: {}", e)),

        Config(opts) => opts
            .config()
            .map_err(|e| anyhow!("🚫 Configuring new project failed: {}", e)),

        #[cfg(feature = "plugin")]
        Plugin(opts) => opts
            .plugin()
            .await
            .map_err(|e| anyhow!("🚫 Error with plugin: {}", e)),

        Autoformat(opts) => opts
            .autoformat()
            .await
            .map_err(|e| anyhow!("🚫 Error autoformatting RSX: {}", e)),

        Version(opt) => {
            let version = opt.version();
            println!("{}", version);

            Ok(())
        }
    }
}
