use dioxus_cli::*;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
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

        // Commands::Clean(_) => {
        //     //
        // }

        // Commands::Config(_) => {
        //     //
        // }
        Commands::Serve(opts) => {
            if let Err(e) = opts.serve().await {
                log::error!("serve error: {}", e);
            }
        }

        Commands::Init(opts) => {
            if let Err(e) = opts.init() {
                log::error!("init error: {}", e);
            }
        }
    }

    Ok(())
}
