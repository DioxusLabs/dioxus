
use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
pub struct LaunchOptions {
    #[argh(subcommand)]
    pub command: LaunchCommand,
}

/// The various kinds of commands that `wasm-pack` can execute.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum LaunchCommand {
    Develop(DevelopOptions),
    Build(BuildOptions),
    Test(TestOptions),
    Publish(PublishOptions),
}

/// Publish your yew application to Github Pages, Netlify, or S3
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "publish")]
pub struct PublishOptions {}

/// 🔬 test your yew application!
#[derive(FromArgs, PartialEq, Debug)]
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
    /// an optional direction which is "up" by default
    #[argh(option, short = 'o', default = "String::from(\"public\")")]
    pub outdir: String,

    /// an example in the crate
    #[argh(option)]
    pub example: Option<String>,
}

/// 🛠 Start a development server
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "develop")]
pub struct DevelopOptions {
    /// an example in the crate
    #[argh(option)]
    pub example: Option<String>,
}
