use std::path::PathBuf;

use anyhow::anyhow;
use clap::Parser;
use dioxus_cli::*;

#[cfg(feature = "plugin")]
use dioxus_cli::plugin::PluginManager;

use Commands::*;

fn get_bin(bin: Option<String>) -> Result<Option<PathBuf>> {
    const ERR_MESSAGE: &str = "The `--bin` flag has to be ran in a Cargo workspace.";

    if let Some(ref bin) = bin {
        let manifest = cargo_toml::Manifest::from_path("./Cargo.toml")
            .map_err(|_| Error::CargoError(ERR_MESSAGE.to_string()))?;

        if let Some(workspace) = manifest.workspace {
            for item in workspace.members.iter() {
                let path = PathBuf::from(item);

                if !path.exists() {
                    continue;
                }

                if !path.is_dir() {
                    continue;
                }

                if path.ends_with(bin.clone()) {
                    return Ok(Some(path));
                }
            }
        } else {
            return Err(Error::CargoError(ERR_MESSAGE.to_string()));
        }
    }

    // If the bin exists but we couldn't find it
    if bin.is_some() {
        return Err(Error::CargoError(
            "The specified bin does not exist.".to_string(),
        ));
    }

    Ok(None)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    set_up_logging();

    let bin = get_bin(args.bin)?;

    let _dioxus_config = DioxusConfig::load(bin.clone())
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
            .build(bin.clone())
            .map_err(|e| anyhow!("🚫 Building project failed: {}", e)),

        Clean(opts) => opts
            .clean(bin.clone())
            .map_err(|e| anyhow!("🚫 Cleaning project failed: {}", e)),

        Serve(opts) => opts
            .serve(bin.clone())
            .await
            .map_err(|e| anyhow!("🚫 Serving project failed: {}", e)),

        Create(opts) => opts
            .create()
            .map_err(|e| anyhow!("🚫 Creating new project failed: {}", e)),

        Config(opts) => opts
            .config()
            .map_err(|e| anyhow!("🚫 Configuring new project failed: {}", e)),

        Bundle(opts) => opts
            .bundle(bin.clone())
            .map_err(|e| anyhow!("🚫 Bundling project failed: {}", e)),

        #[cfg(feature = "plugin")]
        Plugin(opts) => opts
            .plugin()
            .await
            .map_err(|e| anyhow!("🚫 Error with plugin: {}", e)),

        Autoformat(opts) => opts
            .autoformat()
            .await
            .map_err(|e| anyhow!("🚫 Error autoformatting RSX: {}", e)),

        Check(opts) => opts
            .check()
            .await
            .map_err(|e| anyhow!("🚫 Error checking RSX: {}", e)),

        Version(opt) => {
            let version = opt.version();
            println!("{}", version);

            Ok(())
        }
    }
}
