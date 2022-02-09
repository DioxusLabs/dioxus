use super::*;


/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "config")]
pub enum Config {
    /// Init `Dioxus.toml` for project/folder.
    Init {
        /// Init project name
        name: String,

        /// Cover old config
        #[clap(long)]
        #[serde(default)]
        force: bool,
    },
}

impl Config {
    pub fn config(self) -> Result<()> {
        let crate_root = crate::cargo::crate_root()?;
        match self {
            Config::Init { name, force } => {
                let conf_path = crate_root.join("Dioxus.toml");
                if conf_path.is_file() && !force {
                    log::warn!(
                        "config file `Dioxus.toml` already exist, use `--force` to overwrite it."
                    );
                    return Ok(());
                }
                let mut file = File::create(conf_path)?;
                let content = String::from(include_str!("../../assets/dioxus.toml"))
                    .replace("{{project-name}}", &name);
                file.write_all(content.as_bytes())?;
                log::info!("🚩 Init config file completed.");
            }
        }
        Ok(())
    }
}
