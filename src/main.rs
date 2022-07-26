use clap::Parser;
use dioxus_cli::{*, plugin::{PluginManager, PluginConfig}};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    set_up_logging();

    let plugin_manager = PluginManager::init(&PluginConfig {
        available: true,
        required: vec![],
    }).unwrap();

    plugin_manager.load_all_plugins().unwrap();

    match args.action {
        Commands::Translate(opts) => {
            if let Err(e) = opts.translate() {
                log::error!("🚫 Translate failed: {}", e);
            }
        }

        Commands::Build(opts) => {
            if let Err(e) = opts.build() {
                log::error!("🚫 Build project failed: {}", e);
            }
        }

        Commands::Clean(opts) => {
            if let Err(e) = opts.clean() {
                log::error!("🚫 Clean project failed: {}", e);
            }
        }

        Commands::Serve(opts) => {
            if let Err(e) = opts.serve().await {
                log::error!("🚫 Serve startup failed: {}", e);
            }
        }

        Commands::Create(opts) => {
            if let Err(e) = opts.create() {
                log::error!("🚫 Create project failed: {}", e);
            }
        }

        Commands::Config(opts) => {
            if let Err(e) = opts.config() {
                log::error!("config error: {}", e);
            }
        }

        Commands::Tool(opts) => {
            if let Err(e) = opts.tool().await {
                log::error!("tool error: {}", e);
            }
        }
    }

    Ok(())
}
