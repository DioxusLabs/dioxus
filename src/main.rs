use clap::Parser;
use dioxus_cli::*;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    set_up_logging();

    match args.action {
        Commands::Translate(opts) => {
            if let Err(e) = opts.translate() {
                log::error!("translate error: {}", e);
            }
        }

        Commands::Build(opts) => {
            if let Err(e) = opts.build() {
                log::error!("build error: {}", e);
            }
        }

        Commands::Clean(opts) => {
            if let Err(e) = opts.clean() {
                log::error!("clean error: {}", e);
            }
        }

        Commands::Serve(opts) => {
            if let Err(e) = opts.serve().await {
                log::error!("serve error: {}", e);
            }
        }

        Commands::Create(opts) => {
            if let Err(e) = opts.create() {
                log::error!("create error: {}", e);
            }
        }

        Commands::Config(opts) => {
            if let Err(e) = opts.config() {
                log::error!("config error: {}", e);
            }
        }
    }

    Ok(())
}
