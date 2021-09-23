use dioxus_studio as diopack;
use dioxus_studio::cli::{LaunchCommand, LaunchOptions};

#[async_std::main]
async fn main() -> diopack::error::Result<()> {
    diopack::logging::set_up_logging();

    let opts: LaunchOptions = argh::from_env();
    let mut config = diopack::config::Config::new()?;

    match opts.command {
        LaunchCommand::Build(options) => {
            config.with_build_options(&options);
            diopack::builder::build(&config, &(options.into()))?;
        }

        LaunchCommand::Develop(options) => {
            config.with_develop_options(&options);
            diopack::develop::start(&config, &(options.into())).await?;
        }

        _ => {
            todo!("Command not currently implemented");
        }
    }

    Ok(())
}
