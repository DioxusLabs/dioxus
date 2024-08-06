use super::*;
use cargo_generate::{GenerateArgs, TemplatePath};
use cargo_metadata::Metadata;
use std::path::Path;

pub(crate) static DEFAULT_TEMPLATE: &str = "gh:dioxuslabs/dioxus-template";

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "new")]
pub struct Create {
    /// Project name (required when `--yes` is used)
    name: Option<String>,

    /// Generate the template directly at the given path.
    #[arg(long, value_parser)]
    destination: Option<PathBuf>,

    /// Generate the template directly into the current dir. No subfolder will be created and no vcs is initialized.
    #[arg(long, action)]
    init: bool,

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
}

impl Create {
    pub fn create(mut self) -> Result<()> {
        let metadata = cargo_metadata::MetadataCommand::new().exec().ok();

        // If we're getting pass a `.` name, that's actually a path
        // We're actually running an init - we should clear the name
        if self.name.as_deref() == Some(".") {
            self.name = None;
            self.init = true;
        }

        // A default destination is set for nameless projects
        if self.name.is_none() {
            self.destination = Some(PathBuf::from("."));
        }

        // Split the name into path components
        // such that dx new packages/app will create a directory called packages/app
        let destination = self.destination.unwrap_or_else(|| {
            let mut path = PathBuf::from(self.name.as_deref().unwrap());

            if path.is_relative() {
                path = std::env::current_dir().unwrap().join(path);
            }

            // split the path into the parent and the name
            let parent = path.parent().unwrap();
            let name = path.file_name().unwrap();
            self.name = Some(name.to_str().unwrap().to_string());

            // create the parent directory if it doesn't exist
            std::fs::create_dir_all(parent).unwrap();

            // And then the "destination" is the parent directory
            parent.to_path_buf()
        });

        let args = GenerateArgs {
            define: self.option,
            name: self.name,
            silent: self.yes,
            template_path: TemplatePath {
                auto_path: Some(self.template),
                subfolder: self.subtemplate,
                ..Default::default()
            },
            init: self.init,
            destination: Some(destination),
            vcs: if metadata.is_some() {
                Some(cargo_generate::Vcs::None)
            } else {
                None
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

        post_create(&path, metadata)
    }
}

/// Post-creation actions for newly setup crates.
// Also used by `init`.
pub fn post_create(path: &Path, metadata: Option<Metadata>) -> Result<()> {
    // 1. Add the new project to the workspace, if it exists.
    //    This must be executed first in order to run `cargo fmt` on the new project.
    metadata.and_then(|metadata| {
        let cargo_toml_path = &metadata.workspace_root.join("Cargo.toml");
        let cargo_toml_str = std::fs::read_to_string(cargo_toml_path).ok()?;
        let relative_path = path.strip_prefix(metadata.workspace_root).ok()?;

        let mut cargo_toml: toml_edit::DocumentMut = cargo_toml_str.parse().ok()?;
        cargo_toml
            .get_mut("workspace")?
            .get_mut("members")?
            .as_array_mut()?
            .push(relative_path.display().to_string());

        std::fs::write(cargo_toml_path, cargo_toml.to_string()).ok()
    });

    // 2. Run `cargo fmt` on the new project.
    let mut cmd = Command::new("cargo");
    let cmd = cmd.arg("fmt").current_dir(path);
    let output = cmd.output().expect("failed to execute process");
    if !output.status.success() {
        tracing::error!("cargo fmt failed");
        tracing::error!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        tracing::error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    // 3. Format the `Cargo.toml` and `Dioxus.toml` files.
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

    // 4. Remove any triple newlines from the readme.
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
