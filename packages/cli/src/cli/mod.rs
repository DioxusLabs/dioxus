pub(crate) mod autoformat;
pub(crate) mod build;
pub(crate) mod bundle;
pub(crate) mod check;
pub(crate) mod clean;
pub(crate) mod config;
pub(crate) mod create;
pub(crate) mod init;
pub(crate) mod link;
pub(crate) mod run;
pub(crate) mod serve;
pub(crate) mod target;
pub(crate) mod translate;
pub(crate) mod update;
pub(crate) mod verbosity;

pub(crate) use build::*;
pub(crate) use serve::*;
pub(crate) use target::*;
pub(crate) use verbosity::*;

use crate::{error::Result, Error, StructuredOutput};
use clap::builder::styling::{AnsiColor, Effects, Style, Styles};
use clap::{Parser, Subcommand};
use html_parser::Dom;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    process::Command,
};

/// Dioxus: build web, desktop, and mobile apps with a single codebase.
///
/// ## Creating a New Project
///
/// You can use `dx new` to create a new dioxus project. The CLI will ask you a few questions about your project and then create a new project for you:
/// ```sh
/// dx new my-app
/// ```
///
/// ## Serving Your App
///
/// You can use `dx serve` to serve your dioxus app. This will start a local server and watch for changes to your app:
/// ```sh
/// dx serve
/// ```
///
/// ## Bundling Your App
///
/// Once you are ready to ship your app, you can use `dx bundle` to build your app. This will create a production-ready build of your app:
/// ```sh
/// dx bundle
/// ```
///
/// ## Asset Optimizer
///
/// When you serve dioxus with dx, it will automatically handle the build process for you. If you need to integrate with a larger build system,
/// you can use the `dx` asset optimizer separately to link to your assets. If you set the `DX_LINK_ASSETS_TARGET` environment variable, dx will
/// proxy your linker and copy the optimized assets it finds in your program into the specified directory.
///
/// ### Usage with trunk
///
/// If you are using trunk, you need to create a temporary asset directory to store the output of the dx asset optimizer that will be copied by trunk into your dist directory:
/// ```html
/// <html>
///   <head>
///     <link data-trunk rel="rust"/>
///     <link data-trunk rel="copy-dir" href="./dist_assets/" data-target-path="./assets/"/>
///   </head>
///   <body>
///     <div id="main"></div>
///   </body>
/// </html>
/// ```
/// Then when you build, you need to set the `DX_LINK_ASSETS_TARGET` environment variable to the path of the temporary asset directory and `dx` as your linker:
/// ```sh
/// DX_LINK_ASSETS_TARGET="dist_assets" RUSTFLAGS="-Clinker=dx" trunk serve
/// ```
///
/// ### Usage with cargo
///
/// If you are using cargo, you need to set the `DX_LINK_ASSETS_TARGET` environment variable to the path where your optimize assets will be stored and `dx` as your linker:
/// ```sh
/// DX_LINK_ASSETS_TARGET="dist_assets" RUSTFLAGS="-Clinker=dx" cargo run
/// ```
///
/// ### Custom linker path
///
/// DX will try to find the default linker for your system, but if you need to use a custom linker on top of the dx proxy, you can set the `DX_LINK_CUSTOM_LINKER` environment variable to the path of your custom linker. For example, if you are using `lld` as your linker, you can set the `DX_LINK_CUSTOM_LINKER` environment variable to the path of `lld`:
/// ```sh
/// DX_LINK_CUSTOM_LINKER="/path/to/lld" DX_LINK_ASSETS_TARGET="dist_assets" RUSTFLAGS="-Clinker=dx" cargo run
/// ```
#[derive(Parser)]
#[clap(name = "dioxus", version = VERSION.as_str())]
#[clap(styles = CARGO_STYLING)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) action: Commands,

    #[command(flatten)]
    pub(crate) verbosity: Verbosity,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Create a new Dioxus project.
    #[clap(name = "new")]
    New(create::Create),

    /// Build, watch, and serve the project.
    #[clap(name = "serve")]
    Serve(serve::ServeArgs),

    /// Bundle the Dioxus app into a shippable object.
    #[clap(name = "bundle")]
    Bundle(bundle::Bundle),

    /// Build the Dioxus project and all of its assets.
    #[clap(name = "build")]
    Build(build::BuildArgs),

    /// Run the project without any hotreloading.
    #[clap(name = "run")]
    Run(run::RunArgs),

    /// Init a new project for Dioxus in the current directory (by default).
    /// Will attempt to keep your project in a good state.
    #[clap(name = "init")]
    Init(init::Init),

    /// Clean output artifacts.
    #[clap(name = "clean")]
    Clean(clean::Clean),

    /// Translate a source file into Dioxus code.
    #[clap(name = "translate")]
    Translate(translate::Translate),

    /// Automatically format RSX.
    #[clap(name = "fmt")]
    Autoformat(autoformat::Autoformat),

    /// Check the project for any issues.
    #[clap(name = "check")]
    Check(check::Check),

    /// Dioxus config file controls.
    #[clap(subcommand)]
    #[clap(name = "config")]
    Config(config::Config),

    /// Update the Dioxus CLI to the latest version.
    #[clap(name = "self-update")]
    SelfUpdate(update::SelfUpdate),
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::Build(_) => write!(f, "build"),
            Commands::Translate(_) => write!(f, "translate"),
            Commands::Serve(_) => write!(f, "serve"),
            Commands::New(_) => write!(f, "create"),
            Commands::Init(_) => write!(f, "init"),
            Commands::Clean(_) => write!(f, "clean"),
            Commands::Config(_) => write!(f, "config"),
            Commands::Autoformat(_) => write!(f, "fmt"),
            Commands::Check(_) => write!(f, "check"),
            Commands::Bundle(_) => write!(f, "bundle"),
            Commands::Run(_) => write!(f, "run"),
            Commands::SelfUpdate(_) => write!(f, "self-update"),
        }
    }
}

pub(crate) static VERSION: Lazy<String> = Lazy::new(|| {
    format!(
        "{} ({})",
        crate::dx_build_info::PKG_VERSION,
        crate::dx_build_info::GIT_COMMIT_HASH_SHORT.unwrap_or("was built without git repository")
    )
});

/// Cargo's color style
/// [source](https://github.com/crate-ci/clap-cargo/blob/master/src/style.rs)
pub(crate) const CARGO_STYLING: Styles = Styles::styled()
    .header(styles::HEADER)
    .usage(styles::USAGE)
    .literal(styles::LITERAL)
    .placeholder(styles::PLACEHOLDER)
    .error(styles::ERROR)
    .valid(styles::VALID)
    .invalid(styles::INVALID);

pub mod styles {
    use super::*;
    pub(crate) const HEADER: Style = AnsiColor::Green.on_default().effects(Effects::BOLD);
    pub(crate) const USAGE: Style = AnsiColor::Green.on_default().effects(Effects::BOLD);
    pub(crate) const LITERAL: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
    pub(crate) const PLACEHOLDER: Style = AnsiColor::Cyan.on_default();
    pub(crate) const ERROR: Style = AnsiColor::Red.on_default().effects(Effects::BOLD);

    pub(crate) const VALID: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
    pub(crate) const INVALID: Style = AnsiColor::Yellow.on_default().effects(Effects::BOLD);

    // extra styles for styling logs
    // we can style stuff using the ansi sequences like: "hotpatched in {GLOW_STYLE}{}{GLOW_STYLE:X}ms"
    pub(crate) const GLOW_STYLE: Style = AnsiColor::Yellow.on_default();
    pub(crate) const NOTE_STYLE: Style = AnsiColor::Green.on_default();
    pub(crate) const LINK_STYLE: Style = AnsiColor::Blue.on_default();
}
