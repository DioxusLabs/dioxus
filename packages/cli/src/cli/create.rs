use super::*;
use cargo_generate::{GenerateArgs, TemplatePath};

pub(crate) static DEFAULT_TEMPLATE: &str = "gh:dioxuslabs/dioxus-template";

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "new")]
pub struct Create {
    /// Project name (required when `--yes` is used)
    name: Option<String>,
    /// Template path
    #[clap(default_value = DEFAULT_TEMPLATE, short, long)]
    template: String,
    /// Pass <option>=<value> for the used template (e.g., `foo=bar`)
    #[clap(short, long)]
    option: Vec<String>,
    /// Specify a sub-template within the template repository to be used as the actual template
    #[clap(long)]
    subtemplate: Option<String>,
    /// Skip user interaction by using the default values for the used template.
    /// Default values can be overridden with `--option`
    #[clap(short, long)]
    yes: bool,
    // TODO: turn on/off cargo-generate's output (now is invisible)
    // #[clap(default_value = "false", short, long)]
    // silent: bool,
}

impl Create {
    pub fn create(self) -> Result<()> {
        let args = GenerateArgs {
            define: self.option,
            name: self.name,
            silent: self.yes,
            template_path: TemplatePath {
                auto_path: Some(self.template),
                subfolder: self.subtemplate,
                ..Default::default()
            },
            ..Default::default()
        };
        if self.yes && args.name.is_none() {
            return Err("You have to provide the project's name when using `--yes` option.".into());
        }
        // https://github.com/console-rs/dialoguer/issues/294
        ctrlc::set_handler(move || {
            let _ = console::Term::stdout().show_cursor();
            std::process::exit(0);
        })
        .expect("ctrlc::set_handler");
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
        tracing::error!("cargo fmt failed");
        tracing::error!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        tracing::error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    // then format the toml
    let toml_paths = [path.join("Cargo.toml"), path.join("Dioxus.toml")];
    for toml_path in &toml_paths {
        let toml = std::fs::read_to_string(toml_path)?;
        let mut toml = toml.parse::<toml_edit::DocumentMut>().map_err(|e| {
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

    tracing::info!("Generated project at {}", path.display());

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
