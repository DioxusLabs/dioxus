use dioxus_cli::*;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    set_up_logging();

    match args.action {
        Commands::Translate(opts) => {
            opts.translate();
        }

        // Commands::Build(_) => {
        //     //
        // }


        // Commands::Clean(_) => {
        //     //
        // }

        // Commands::Config(_) => {
        //     //
        // }

        // Commands::Serve(_) => {
        //     //
        // }
    }

    Ok(())
}
