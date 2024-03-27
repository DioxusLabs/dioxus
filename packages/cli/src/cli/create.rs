use std::process::exit;

use super::*;
use cargo_generate::{GenerateArgs, TemplatePath};

static DEFAULT_TEMPLATE: &str = "gh:dioxuslabs/dioxus-template";

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "new")]
pub struct Create {
    /// Template path
    #[clap(default_value = DEFAULT_TEMPLATE, short, long)]
    template: String,
    /// Project name (required when `--yes` is used)
    #[clap(short, long)]
    name: Option<String>,
    /// Pass `<option>=<value>` for the used template
    ///
    /// Options for the default template:
    /// - platform: target platform [default: desktop]
    ///   * desktop
    ///   * web
    ///   * TUI
    ///   * Liveview
    ///   * Fullstack
    ///   Note: this option is required
    /// - backend: which backend framework to use [default: Axum]
    ///   * Axum:  use Axum
    ///   * Warp:  use Warp
    ///   * Salvo: use Salvo
    ///   Note: only used when platform is equal to: Fullstack, Liveview
    /// - router: Whether to use dioxus router or not [default: true]
    ///   * true:  use Dioxus router
    ///   * false: don't use Dioxus router
    /// - styling: CSS creation method [default: Vanilla]
    ///   * Vanilla:  regular CSS
    ///   * Tailwind: Tailwind CSS
    ///   Note: only used when platform is one of: web, desktop, Fullstack
    #[clap(short, long, verbatim_doc_comment)]
    option: Vec<String>,
    /// Skip user interaction by using the default values for the used template.
    /// Default values can be overriden with `--option`
    #[clap(default_value = "false", short, long)]
    yes: bool,
}

impl Create {
    pub fn create(self) -> Result<()> {
        let mut args = GenerateArgs {
            template_path: TemplatePath {
                auto_path: Some(self.template),
                ..Default::default()
            },
            ..Default::default()
        };
        if self.yes {
            args.silent = true;
            args.define = self.option;
            args.name = self.name.or_else(|| {
                log::error!("You have to provide the project's name with `--name` when using `--yes` option.");
                exit(1); // Do we have a list of exit codes for `dx`?
                // CLI commands shouldn't say "thread 'main' panicked" if we know why it panicked.
                // panic!("You have to provide the project's name when using `--yes` option.")
            });
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
        tracing::error!("cargo fmt failed");
        tracing::error!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        tracing::error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
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
