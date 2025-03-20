use clap::{ArgMatches, Args, FromArgMatches, Parser, Subcommand};
use serde::{de::DeserializeOwned, Deserialize};

// https://github.com/clap-rs/clap/issues/2222#issuecomment-2524152894
//
//
/// `[Args]` wrapper to match `T` variants recursively in `U`.
#[derive(Debug, Clone)]
pub struct ChainedCommand<T, U> {
    /// Specific Variant.
    inner: T,

    /// Enum containing `Self<T>` variants, in other words possible follow-up commands.
    next: Option<Box<U>>,
}

impl<T, U> ChainedCommand<T, U>
where
    T: Args,
    U: Subcommand,
{
    fn commands(self) -> Vec<Self> {
        todo!()
    }
}

impl<T, U> Args for ChainedCommand<T, U>
where
    T: Args,
    U: Subcommand,
{
    fn augment_args(cmd: clap::Command) -> clap::Command {
        // We use the special `defer` method whcih lets us recursively call `augment_args` on the inner command
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
            // we are done, since sub-sub commmands are matched in U::
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

impl<'de, T: Deserialize<'de>, U: Deserialize<'de>> Deserialize<'de> for ChainedCommand<T, U> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    #[derive(Debug, Parser)]
    struct TestCli {
        #[clap(long)]
        top: Option<String>,

        #[command(subcommand)]
        cmd: TopCmd,
    }

    /// Launch a specific target
    ///
    /// You can specify multiple targets using `@client --args` syntax.
    #[derive(Debug, Parser)]
    struct ServeCommand {
        #[clap(flatten)]
        args: Target,

        #[command(subcommand)]
        targets: TopCmd,
    }

    #[derive(Debug, Subcommand, Clone)]
    enum TopCmd {
        Serve {
            #[clap(subcommand)]
            cmd: Cmd,
        },
    }

    /// Launch a specific target
    #[derive(Debug, Subcommand, Clone)]
    #[command(subcommand_precedence_over_arg = true)]
    enum Cmd {
        /// Specify the arguments for the client build
        #[clap(name = "client")]
        Client(ReClap<Target, Self>),

        /// Specify the arguments for the server build
        #[clap(name = "server")]
        Server(ReClap<Target, Self>),

        /// Specify the arguments for any number of additional targets
        #[clap(name = "target")]
        Target(ReClap<Target, Self>),
    }

    #[derive(Clone, Args, Debug)]
    struct Target {
        #[arg(short, long)]
        profile: Option<String>,

        #[arg(short, long)]
        target: Option<String>,

        #[arg(short, long)]
        bin: Option<String>,
    }

    #[test]
    fn test_parse_args() {
        let args = r#"
dx serve
    @client --release
    @server --target wasm32
    @target --bin mybin
    @target --bin mybin
    @target --bin mybin
    @target --bin mybin
"#
        .trim()
        .split_ascii_whitespace();

        let cli = TestCli::parse_from(args);

        dbg!(&cli);

        match cli.cmd {
            TopCmd::Serve { cmd } => {
                let mut next = Some(cmd);

                // let mut next = cmd.cmd;
                while let Some(cmd) = next {
                    // println!("{cmd:?}");
                    // could use enum_dispatch
                    next = match cmd {
                        Cmd::Client(rec) => {
                            //
                            (rec.next).map(|d| *d)
                        }
                        Cmd::Server(rec) => {
                            //
                            (rec.next).map(|d| *d)
                        }
                        Cmd::Target(rec) => {
                            //
                            (rec.next).map(|d| *d)
                        }
                    }
                }
            }
        }
    }
}
