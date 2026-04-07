use super::*;
use crate::{AddressArguments, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

/// Generate shell completions for the specified shell.
#[derive(Clone, Debug, Parser)]
pub(crate) struct ShellCompletions {
    /// The shell to generate completions for.
    #[clap(value_enum)]
    pub shell: Shell,
}

impl ShellCompletions {
    pub fn generate_and_print(self) -> Result<StructuredOutput> {
        let mut cmd = CompletionCli::command();
        generate(
            self.shell,
            &mut cmd,
            env!("CARGO_BIN_NAME"),
            &mut std::io::stdout(),
        );
        Ok(StructuredOutput::Success)
    }
}

// Completion generation eagerly expands the entire clap command tree.
//
// Our real CLI uses recursive deferred subcommands to support chained `@client` / `@server`
// platform overrides, which is fine for parsing but causes completion generation to recurse
// without a terminal leaf. We use a finite mirror of the CLI here so `clap_complete` can walk
// the command tree safely.
#[derive(Parser)]
#[clap(name = "dioxus", version = VERSION.as_str())]
#[clap(about = "Dioxus: build web, desktop, and mobile apps with a single codebase")]
#[clap(styles = CARGO_STYLING)]
struct CompletionCli {
    #[command(subcommand)]
    action: CompletionCommands,

    #[command(flatten)]
    verbosity: Verbosity,
}

#[derive(Subcommand)]
enum CompletionCommands {
    #[clap(name = "new")]
    New(create::Create),

    #[clap(name = "serve")]
    Serve(CompletionServeArgs),

    #[clap(name = "bundle")]
    Bundle(CompletionBundle),

    #[clap(name = "build")]
    Build(CompletionCommandWithPlatformOverrides<build::BuildArgs>),

    #[clap(name = "run")]
    Run(CompletionRunArgs),

    #[clap(name = "init")]
    Init(init::Init),

    #[clap(name = "doctor")]
    Doctor(doctor::Doctor),

    #[clap(name = "completions")]
    ShellCompletions(ShellCompletions),

    #[clap(name = "print")]
    #[clap(subcommand)]
    Print(CompletionPrint),

    #[clap(name = "translate")]
    Translate(translate::Translate),

    #[clap(name = "fmt")]
    Autoformat(autoformat::Autoformat),

    #[clap(name = "check")]
    Check(CompletionCheck),

    #[clap(subcommand)]
    #[clap(name = "config")]
    Config(config::Config),

    #[clap(name = "self-update")]
    SelfUpdate(update::SelfUpdate),

    #[clap(name = "tools")]
    #[clap(subcommand)]
    Tools(CompletionBuildTools),

    #[clap(name = "components")]
    #[clap(subcommand)]
    Components(component::ComponentCommand),
}

#[derive(Subcommand)]
enum CompletionBuildTools {
    #[clap(name = "assets")]
    BuildAssets(build_assets::BuildAssets),

    #[clap(name = "hotpatch")]
    HotpatchTip(CompletionHotpatchTip),
}

#[derive(Clone, Debug, Args)]
#[command(disable_help_subcommand = true)]
struct CompletionCommandWithPlatformOverrides<T: Args> {
    #[command(flatten)]
    shared: T,

    #[command(subcommand)]
    override_command: Option<CompletionPlatformOverride<T>>,
}

#[derive(Clone, Debug, Subcommand)]
#[command(subcommand_precedence_over_arg = true)]
enum CompletionPlatformOverride<T: Args> {
    #[clap(name = "@client")]
    Client(CompletionPlatformOverrideArgs<T>),

    #[clap(name = "@server")]
    Server(CompletionPlatformOverrideArgs<T>),
}

#[derive(Clone, Debug, Args)]
#[command(disable_help_subcommand = true)]
struct CompletionPlatformOverrideArgs<T: Args> {
    #[command(flatten)]
    args: T,

    #[command(subcommand)]
    next: Option<CompletionPlatformOverrideLeaf<T>>,
}

#[derive(Clone, Debug, Subcommand)]
#[command(subcommand_precedence_over_arg = true)]
enum CompletionPlatformOverrideLeaf<T: Args> {
    #[clap(name = "@client")]
    Client(CompletionPlatformOverrideLeafArgs<T>),

    #[clap(name = "@server")]
    Server(CompletionPlatformOverrideLeafArgs<T>),
}

#[derive(Clone, Debug, Args)]
struct CompletionPlatformOverrideLeafArgs<T: Args> {
    #[command(flatten)]
    args: T,
}

#[derive(Clone, Debug, Parser)]
#[command(group = clap::ArgGroup::new("release-incompatible").multiple(true).conflicts_with("release"))]
struct CompletionServeArgs {
    #[clap(flatten)]
    address: AddressArguments,

    #[arg(long, default_missing_value = "true", num_args = 0..=1)]
    open: Option<bool>,

    #[clap(long, group = "release-incompatible")]
    hot_reload: Option<bool>,

    #[clap(long, default_missing_value = "true")]
    always_on_top: Option<bool>,

    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    cross_origin_policy: bool,

    #[clap(long, default_missing_value = "2")]
    wsl_file_poll_interval: Option<u16>,

    #[arg(long, default_missing_value = "true", num_args = 0..=1, short = 'i')]
    interactive: Option<bool>,

    #[arg(long, default_value_t = false, alias = "hotpatch")]
    hot_patch: bool,

    #[clap(long, default_missing_value = "true", num_args = 0..=1)]
    watch: Option<bool>,

    #[clap(long)]
    #[clap(hide = true)]
    exit_on_error: bool,

    #[clap(flatten)]
    platform_args: CompletionCommandWithPlatformOverrides<serve::PlatformServeArgs>,
}

#[derive(Clone, Debug, Parser)]
struct CompletionRunArgs {
    #[clap(flatten)]
    args: CompletionServeArgs,
}

#[derive(Clone, Debug, Parser)]
struct CompletionBundle {
    #[clap(long)]
    package_types: Option<Vec<crate::PackageType>>,

    #[clap(long)]
    out_dir: Option<std::path::PathBuf>,

    #[clap(flatten)]
    args: CompletionCommandWithPlatformOverrides<build::BuildArgs>,
}

#[derive(Clone, Debug, Parser)]
struct CompletionCheck {
    #[clap(short, long)]
    file: Option<std::path::PathBuf>,

    #[clap(flatten)]
    build_args: CompletionCommandWithPlatformOverrides<build::BuildArgs>,
}

#[derive(Clone, Debug, Subcommand)]
enum CompletionPrint {
    #[clap(name = "client-args")]
    ClientArgs(CompletionPrintCargoArgs),

    #[clap(name = "server-args")]
    ServerArgs(CompletionPrintCargoArgs),
}

#[derive(Clone, Debug, Parser)]
struct CompletionPrintCargoArgs {
    #[clap(flatten)]
    args: CompletionCommandWithPlatformOverrides<build::BuildArgs>,

    #[clap(long)]
    style: Option<print::PrintStyle>,
}

#[derive(Clone, Debug, Parser)]
struct CompletionHotpatchTip {
    #[clap(long, num_args = 0..=1, default_missing_value = "true", help_heading = "Hotpatching a binary")]
    patch_server: Option<bool>,

    #[clap(long, help_heading = "Hotpatching a binary")]
    aslr_reference: u64,

    #[clap(flatten)]
    build_args: CompletionCommandWithPlatformOverrides<build::BuildArgs>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_generate_without_recursing_forever() {
        let mut cmd = CompletionCli::command();
        let mut output = Vec::new();

        clap_complete::generate(Shell::Bash, &mut cmd, "dx", &mut output);

        let output = String::from_utf8(output).expect("completions should be valid utf-8");
        assert!(output.contains("@client"));
        assert!(output.contains("@server"));
        assert!(output.contains("build"));
    }
}
