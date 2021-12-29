mod builder;
mod cargo;
mod cli;
mod config;
mod error;
mod logging;
mod helpers {
    pub mod extract_svgs;
    pub mod to_component;
    pub mod translate;
}

mod watch;
mod develop {
    pub mod develop;
    pub mod draw;
    pub mod events;
    pub mod studio;
}

use std::path::PathBuf;
use structopt::StructOpt;

#[async_std::main]
async fn main() -> Result<()> {
    set_up_logging();
    let args = Args::from_args();

    match args.command {
        LaunchCommand::Develop(cfg) => {
            develop::develop::develop(cfg).await?;
        }
        // LaunchCommand::Develop(cfg) => develop::studio::start(cfg).await?,
        LaunchCommand::Build(opts) => {
            let mut cfg = CrateConfig::new()?;
            cfg.with_build_options(&opts);
            builder::build(&cfg)?;
        }

        LaunchCommand::Translate(cfg) => {
            let TranslateOptions {
                file,
                text,
                component,
            } = cfg;

            match component {
                true => {
                    let f = helpers::to_component::convert_html_to_component(&text.unwrap())?;
                    println!("{}", f);
                }
                false => {
                    let renderer = match (file, text) {
                        (None, Some(text)) => translate::translate_from_html_to_rsx(&text, false)?,
                        (Some(file), None) => translate::translate_from_html_file(&file)?,
                        _ => panic!("Must select either file or text - not both or none!"),
                    };

                    println!("{}", renderer);
                }
            }
        }
        _ => {
            todo!("Those commands are not yet supported");
        }
    }

    Ok(())
}

/// Build, bundle & ship your Rust WASM application to the web.
#[derive(StructOpt)]
#[structopt(name = "trunk")]
struct Args {
    #[structopt(subcommand)]
    command: TrunkSubcommands,
    /// Path to the Trunk config file [default: Trunk.toml]
    #[structopt(long, parse(from_os_str), env = "TRUNK_CONFIG")]
    pub config: Option<PathBuf>,
    /// Enable verbose logging.
    #[structopt(short)]
    pub v: bool,
}

#[derive(StructOpt)]
enum TrunkSubcommands {
    /// Build the Rust WASM app and all of its assets.
    Build(cmd::build::Build),
    /// Build & watch the Rust WASM app and all of its assets.
    Watch(cmd::watch::Watch),
    /// Build, watch & serve the Rust WASM app and all of its assets.
    Serve(cmd::serve::Serve),
    /// Clean output artifacts.
    Clean(cmd::clean::Clean),
    /// Trunk config controls.
    Config(cmd::config::Config),
}
