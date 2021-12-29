use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
pub struct LaunchOptions {
    #[argh(subcommand)]
    pub command: LaunchCommand,
}

/// The various kinds of commands that `wasm-pack` can execute.
#[derive(FromArgs, PartialEq, Debug, Clone)]
#[argh(subcommand)]
pub enum LaunchCommand {
    Develop(DevelopOptions),
    Build(BuildOptions),
    Translate(TranslateOptions),
    Test(TestOptions),
    Publish(PublishOptions),
    Studio(StudioOptions),
}

/// Publish your yew application to Github Pages, Netlify, or S3
#[derive(FromArgs, PartialEq, Debug, Clone)]
#[argh(subcommand, name = "publish")]
pub struct PublishOptions {}

/// 🔬 test your yew application!
#[derive(FromArgs, PartialEq, Debug, Clone)]
#[argh(subcommand, name = "test")]
pub struct TestOptions {
    /// an example in the crate
    #[argh(option)]
    pub example: Option<String>,
}

/// 🏗️  Build your yew application
#[derive(FromArgs, PartialEq, Debug, Clone)]
#[argh(subcommand, name = "build")]
pub struct BuildOptions {
    /// the directory output
    #[argh(option, short = 'o', default = "String::from(\"public\")")]
    pub outdir: String,

    /// an example in the crate
    #[argh(option)]
    pub example: Option<String>,

    /// develop in release mode
    #[argh(switch, short = 'r')]
    pub release: bool,

    /// hydrate the `dioxusroot` element with this content
    #[argh(option, short = 'h')]
    pub hydrate: Option<String>,

    /// custom template
    #[argh(option, short = 't')]
    pub template: Option<String>,
}

/// 🛠 Start a development server
#[derive(FromArgs, PartialEq, Debug, Clone)]
#[argh(subcommand, name = "develop")]
pub struct DevelopOptions {
    /// an example in the crate
    #[argh(option)]
    pub example: Option<String>,

    /// develop in release mode
    #[argh(switch, short = 'r')]
    pub release: bool,

    /// hydrate the `dioxusroot` element with this content
    #[argh(option, short = 'h')]
    pub hydrate: Option<String>,

    /// custom template
    #[argh(option, short = 't')]
    pub template: Option<String>,
}

/// 🛠 Translate some 3rd party template into rsx
#[derive(FromArgs, PartialEq, Debug, Clone)]
#[argh(subcommand, name = "translate")]
pub struct TranslateOptions {
    /// an example in the crate
    #[argh(option, short = 'f')]
    pub file: Option<String>,

    /// an example in the crate
    #[argh(option, short = 't')]
    pub text: Option<String>,

    /// whether or not to jump
    #[argh(switch, short = 'c')]
    pub component: bool,
}
/// 🛠 Translate some 3rd party template into rsx
#[derive(FromArgs, PartialEq, Debug, Clone)]
#[argh(subcommand, name = "studio")]
pub struct StudioOptions {}
