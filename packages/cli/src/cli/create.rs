use super::*;
use cargo_generate::{GenerateArgs, TemplatePath};

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "create")]
pub struct Create {
    /// Template path
    #[clap(default_value = "gh:dioxuslabs/dioxus-template", long)]
    template: String,
}

impl Create {
    pub fn create(self) -> Result<()> {
        let args = GenerateArgs {
            template_path: TemplatePath {
                auto_path: Some(self.template),
                ..Default::default()
            },
            ..Default::default()
        };

        let path = cargo_generate::generate(args)?;

        post_create(&path)
    }
}

// being also used by `init`
pub fn post_create(path: &PathBuf) -> Result<()> {
    // first run cargo fmt
    let mut cmd = Command::new("cargo");
    let cmd = cmd.arg("fmt").current_dir(path);
    let output = cmd.output().expect("failed to execute process");
    if !output.status.success() {
        log::error!("cargo fmt failed");
        log::error!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        log::error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    // then format the toml
    let toml_paths = [path.join("Cargo.toml"), path.join("Dioxus.toml")];
    for toml_path in &toml_paths {
        let toml = std::fs::read_to_string(toml_path)?;
        let mut toml = toml.parse::<toml_edit::Document>().map_err(|e| {
            anyhow::anyhow!(
                "failed to parse toml at {}: {}",
                toml_path.display(),
                e.to_string()
            )
        })?;

        toml.as_table_mut().fmt();

        let as_string = toml.to_string();
        let new_string = remove_triple_newlines(&as_string);
        let mut file = std::fs::File::create(toml_path)?;
        file.write_all(new_string.as_bytes())?;
    }

    // remove any triple newlines from the readme
    let readme_path = path.join("README.md");
    let readme = std::fs::read_to_string(&readme_path)?;
    let new_readme = remove_triple_newlines(&readme);
    let mut file = std::fs::File::create(readme_path)?;
    file.write_all(new_readme.as_bytes())?;

    log::info!("Generated project at {}", path.display());

    Ok(())
}

fn remove_triple_newlines(string: &str) -> String {
    let mut new_string = String::new();
    for char in string.chars() {
        if char == '\n' && new_string.ends_with("\n\n") {
            continue;
        }
        new_string.push(char);
    }
    new_string
}
