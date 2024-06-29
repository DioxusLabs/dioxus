use crate::{assets, error::Result};
use clap::Parser;
use dioxus_cli_config::DioxusConfig;
use std::fs;

#[derive(Clone, Debug, Parser)]
#[clap(name = "link", hide = true)]
pub struct LinkCommand {
    // Allow us to accept any argument after `dx link`
    #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl LinkCommand {
    pub fn link(self) -> Result<()> {
        let Some((working_dir, object_files)) = manganis_cli_support::linker_intercept(self.args)
        else {
            tracing::warn!("Invalid linker arguments.");
            return Ok(());
        };

        let dioxus_config = DioxusConfig::load(None)?.unwrap_or_default();

        // Parse object files, deserialize JSON, & create a file to propogate JSON.
        let json = manganis_cli_support::get_json_from_object_files(object_files);
        let parsed = serde_json::to_string(&json).unwrap();

        let out_dir = working_dir.join(dioxus_config.application.out_dir);
        fs::create_dir_all(&out_dir).unwrap();

        let path = out_dir.join(assets::MG_JSON_OUT);
        fs::write(path, parsed).unwrap();

        Ok(())
    }

    /// We need to pass the subcommand name to Manganis so this
    /// helps centralize where we set the subcommand "name".
    pub fn command_name() -> String {
        "link".to_string()
    }
}
