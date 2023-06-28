use super::*;
use crate::custom_error;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "create")]
pub struct Create {
    /// Init project name
    #[clap(default_value = ".")]
    name: String,

    /// Template path
    #[clap(default_value = "gh:dioxuslabs/dioxus-template", long)]
    template: String,
}

impl Create {
    pub fn create(self) -> Result<()> {
        if Self::name_valid_check(self.name.clone()) {
            return custom_error!("❗Unsupported project name: '{}'.", &self.name);
        }

        let project_path = PathBuf::from(&self.name);

        if project_path.join("Dioxus.toml").is_file() || project_path.join("Cargo.toml").is_file() {
            return custom_error!("🧨 Folder '{}' is initialized.", &self.name);
        }

        log::info!("🔧 Start: Creating new project '{}'.", self.name);

        let output = Command::new("cargo")
            .arg("generate")
            .arg("--help")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            log::warn!("Tool is not installed: cargo-generate, try to install it.");
            let install_output = Command::new("cargo")
                .arg("install")
                .arg("cargo-generate")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .output()?;
            if !install_output.status.success() {
                return custom_error!("Try to install cargo-generate failed.");
            }
        }

        let generate_output = Command::new("cargo")
            .arg("generate")
            .arg(&self.template)
            .arg("--name")
            .arg(&self.name)
            .arg("--force")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()?;

        if !generate_output.status.success() {
            return custom_error!("Generate project failed. Try to update cargo-generate.");
        }

        let mut dioxus_file = File::open(project_path.join("Dioxus.toml"))?;
        let mut meta_file = String::new();
        dioxus_file.read_to_string(&mut meta_file)?;
        meta_file = meta_file.replace("{{project-name}}", &self.name);
        meta_file = meta_file.replace("{{default-platform}}", "web");
        File::create(project_path.join("Dioxus.toml"))?.write_all(meta_file.as_bytes())?;

        println!();
        log::info!("💡 Project initialized:");
        log::info!("🎯> cd ./{}", self.name);
        log::info!("🎯> dioxus serve");

        Ok(())
    }

    fn name_valid_check(name: String) -> bool {
        let r = Regex::new(r"^[a-zA-Z][a-zA-Z0-9\-_]$").unwrap();
        r.is_match(&name)
    }
}
