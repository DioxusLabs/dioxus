use clap::{ArgMatches, Args, FromArgMatches, Subcommand};

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
