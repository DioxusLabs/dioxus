use crate::cli::*;
use crate::Platform;
use clap::{ArgMatches, Args, FromArgMatches, Subcommand};
use target_lexicon::Triple;

/// A single target to build for
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct TargetArgs {
    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long, value_enum)]
    pub(crate) platform: Option<Platform>,

    /// Build in release mode [default: false]
    #[clap(long, short)]
    #[serde(default)]
    pub(crate) release: bool,

    /// The package to build
    #[clap(short, long)]
    pub(crate) package: Option<String>,

    /// Build a specific binary [default: ""]
    #[clap(long)]
    pub(crate) bin: Option<String>,

    /// Build a specific example [default: ""]
    #[clap(long)]
    pub(crate) example: Option<String>,

    /// Build the app with custom a profile
    #[clap(long)]
    pub(crate) profile: Option<String>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub(crate) features: Vec<String>,

    /// Don't include the default features in the build
    #[clap(long)]
    pub(crate) no_default_features: bool,

    /// Include all features in the build
    #[clap(long)]
    pub(crate) all_features: bool,

    /// Rustc platform triple
    #[clap(long)]
    pub(crate) target: Option<Triple>,

    /// Extra arguments passed to `cargo`
    ///
    /// To see a list of args, run `cargo rustc --help`
    ///
    /// This can include stuff like, "--locked", "--frozen", etc. Note that `dx` sets many of these
    /// args directly from other args in this command.
    #[clap(long)]
    pub(crate) cargo_args: Option<String>,

    /// Extra arguments passed to `rustc`. This can be used to customize the linker, or other flags.
    ///
    /// For example, specifign `dx build --rustc-args "-Clink-arg=-Wl,-blah"` will pass "-Clink-arg=-Wl,-blah"
    /// to the underlying the `cargo rustc` command:
    ///
    /// cargo rustc -- -Clink-arg=-Wl,-blah
    ///
    #[clap(long)]
    pub(crate) rustc_args: Option<String>,

    /// Skip collecting assets from dependencies [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) skip_assets: bool,

    /// Inject scripts to load the wasm and js files for your dioxus app if they are not already present [default: true]
    #[clap(long, default_value_t = true)]
    pub(crate) inject_loading_scripts: bool,

    /// Experimental: Bundle split the wasm binary into multiple chunks based on `#[wasm_split]` annotations [default: false]
    #[clap(long, default_value_t = false)]
    pub(crate) wasm_split: bool,

    /// Generate debug symbols for the wasm binary [default: true]
    ///
    /// This will make the binary larger and take longer to compile, but will allow you to debug the
    /// wasm binary
    #[clap(long, default_value_t = true)]
    pub(crate) debug_symbols: bool,

    /// Are we building for a device or just the simulator.
    /// If device is false, then we'll build for the simulator
    #[clap(long)]
    pub(crate) device: Option<bool>,

    /// The base path the build will fetch assets relative to. This will override the
    /// base path set in the `dioxus` config.
    #[clap(long)]
    pub(crate) base_path: Option<String>,
}

/// Chain together multiple target commands
#[derive(Debug, Subcommand, Clone)]
#[command(subcommand_precedence_over_arg = true)]
pub(crate) enum TargetCmd {
    /// Specify the arguments for the client build
    #[clap(name = "@client")]
    Client(ChainedCommand<TargetArgs, TargetCmd>),

    /// Specify the arguments for the server build
    #[clap(name = "@server")]
    Server(ChainedCommand<TargetArgs, TargetCmd>),
}

// https://github.com/clap-rs/clap/issues/2222#issuecomment-2524152894
//
//
/// `[Args]` wrapper to match `T` variants recursively in `U`.
#[derive(Debug, Clone)]
pub struct ChainedCommand<T, U> {
    /// Specific Variant.
    pub inner: T,

    /// Enum containing `Self<T>` variants, in other words possible follow-up commands.
    pub next: Option<Box<U>>,
}

impl<T, U> Args for ChainedCommand<T, U>
where
    T: Args,
    U: Subcommand,
{
    fn augment_args(cmd: clap::Command) -> clap::Command {
        // We use the special `defer` method which lets us recursively call `augment_args` on the inner command
        // and thus `from_arg_matches`
        T::augment_args(cmd).defer(|cmd| U::augment_subcommands(cmd.disable_help_subcommand(true)))
    }

    fn augment_args_for_update(_cmd: clap::Command) -> clap::Command {
        unimplemented!()
    }
}

impl<T, U> FromArgMatches for ChainedCommand<T, U>
where
    T: Args,
    U: Subcommand,
{
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        // Parse the first command before we try to parse the next one.
        let inner = T::from_arg_matches(matches)?;

        // Try to parse the remainder of the command as a subcommand.
        let next = match matches.subcommand() {
            // Subcommand skips into the matched .subcommand, hence we need to pass *outer* matches, ignoring the inner matches
            // (which in the average case should only match enumerated T)
            //
            // Here, we might want to eventually enable arbitrary names of subcommands if they're prefixed
            // with a prefix like "@" ie `dx serve @dog-app/backend --args @dog-app/frontend --args`
            //
            // we are done, since sub-sub commands are matched in U::
            Some(_) => Some(Box::new(U::from_arg_matches(matches)?)),

            // no subcommand matched, we are done
            None => None,
        };

        Ok(Self { inner, next })
    }

    fn update_from_arg_matches(&mut self, _matches: &ArgMatches) -> Result<(), clap::Error> {
        unimplemented!()
    }
}
