#![allow(dead_code)]
use clap::parser::ValueSource;
use clap::{ArgMatches, Args, CommandFactory, FromArgMatches, Parser, Subcommand};

/// Wraps a component with the subcommands `@server` and `@client` which will let you override the
/// base arguments for the client and server instances.
#[derive(Debug, Clone, Default)]
pub struct CommandWithPlatformOverrides<T> {
    /// The arguments that are shared between the client and server
    pub shared: T,
    /// The merged arguments for the server
    pub server: Option<T>,
    /// The merged arguments for the client
    pub client: Option<T>,
}

impl<T> CommandWithPlatformOverrides<T> {
    pub(crate) fn with_client_or_shared<'a, O>(&'a self, f: impl FnOnce(&'a T) -> O) -> O {
        match &self.client {
            Some(client) => f(client),
            None => f(&self.shared),
        }
    }

    pub(crate) fn with_server_or_shared<'a, O>(&'a self, f: impl FnOnce(&'a T) -> O) -> O {
        match &self.server {
            Some(server) => f(server),
            None => f(&self.shared),
        }
    }
}

impl<T: CommandFactory + Args> Parser for CommandWithPlatformOverrides<T> {}

impl<T: CommandFactory + Args> CommandFactory for CommandWithPlatformOverrides<T> {
    fn command() -> clap::Command {
        T::command()
    }

    fn command_for_update() -> clap::Command {
        T::command_for_update()
    }
}

impl<T> Args for CommandWithPlatformOverrides<T>
where
    T: Args,
{
    fn augment_args(cmd: clap::Command) -> clap::Command {
        T::augment_args(cmd).defer(|cmd| {
            PlatformOverrides::<Self>::augment_subcommands(cmd.disable_help_subcommand(true))
        })
    }

    fn augment_args_for_update(_cmd: clap::Command) -> clap::Command {
        unimplemented!()
    }
}

fn merge_matches<T: Args>(base: &ArgMatches, platform: &ArgMatches) -> Result<T, clap::Error> {
    let mut base = T::from_arg_matches(base)?;

    let mut platform = platform.clone();
    let original_ids: Vec<_> = platform.ids().cloned().collect();
    for arg_id in original_ids {
        let arg_name = arg_id.as_str();
        // Remove any default values from the platform matches
        if platform.value_source(arg_name) == Some(ValueSource::DefaultValue) {
            _ = platform.try_clear_id(arg_name);
        }
    }

    // Then merge the stripped platform matches into the base matches
    base.update_from_arg_matches(&platform)?;

    Ok(base)
}

impl<T> FromArgMatches for CommandWithPlatformOverrides<T>
where
    T: Args,
{
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let mut client = None;
        let mut server = None;
        let mut subcommand = matches.subcommand();
        while let Some((name, sub_matches)) = subcommand {
            match name {
                "@client" => client = Some(sub_matches),
                "@server" => server = Some(sub_matches),
                _ => {}
            }
            subcommand = sub_matches.subcommand();
        }

        let shared = T::from_arg_matches(matches)?;
        let client = client
            .map(|client| merge_matches::<T>(matches, client))
            .transpose()?;
        let server = server
            .map(|server| merge_matches::<T>(matches, server))
            .transpose()?;

        Ok(Self {
            shared,
            server,
            client,
        })
    }

    fn update_from_arg_matches(&mut self, _matches: &ArgMatches) -> Result<(), clap::Error> {
        unimplemented!()
    }
}

#[derive(Debug, Subcommand, Clone)]
#[command(subcommand_precedence_over_arg = true)]
pub(crate) enum PlatformOverrides<T: Args> {
    /// Specify the arguments for the client build
    #[clap(name = "@client")]
    Client(ChainedCommand<T, PlatformOverrides<T>>),

    /// Specify the arguments for the server build
    #[clap(name = "@server")]
    Server(ChainedCommand<T, PlatformOverrides<T>>),
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
    fn from_arg_matches(_: &ArgMatches) -> Result<Self, clap::Error> {
        unimplemented!()
    }

    fn update_from_arg_matches(&mut self, _matches: &ArgMatches) -> Result<(), clap::Error> {
        unimplemented!()
    }
}
