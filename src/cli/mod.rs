use structopt::StructOpt;

pub mod build;
pub mod cfg;
pub mod clean;
pub mod serve;
pub mod translate;

/// Build, bundle, & ship your Dioxus app.
///
///
#[derive(StructOpt)]
#[structopt(name = "dioxus")]
pub struct Cli {
    #[structopt(subcommand)]
    pub action: Commands,

    /// Enable verbose logging.
    #[structopt(short)]
    pub v: bool,
    //
    // // note: dioxus is still roughly compatible with trunk
    // /// Path to the Trunk config file [default: Trunk.toml]
    // #[structopt(long, parse(from_os_str), env = "TRUNK_CONFIG")]
    // pub config: Option<PathBuf>,
}

#[derive(StructOpt)]
pub enum Commands {
    // /// Build the Rust WASM app and all of its assets.
    Build(build::Build),
    /// Translate some source file into Dioxus code.
    Translate(translate::Translate),
    // /// Build, watch & serve the Rust WASM app and all of its assets.
    Serve(serve::Serve),
    // /// Clean output artifacts.
    // Clean(clean::Clean),

    // /// Trunk config controls.
    // Config(config::Config),
}
