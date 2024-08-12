use super::*;
use cargo_generate::{GenerateArgs, TemplatePath};
use std::path::Path;

pub(crate) static DEFAULT_TEMPLATE: &str = "gh:dioxuslabs/dioxus-template";

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "new")]
pub struct Create {
    /// Create a new Dioxus project at specified path (required when `--yes` is used)
    path: Option<PathBuf>,

    /// Project name. Defaults to directory name
    #[arg(short, long)]
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
}

impl Create {
    pub fn create(self) -> Result<()> {
        // TODO: try to get all paths of `yes` `name` `path` options fixed.

        // NOTE: destination \wo init means base_dir + name, \w means dest_dir
        // so use init: true and always handle the dest_dir manually and carefully.
        // Cargo never adds name to the path. Name is solely for project name.

        if self.yes && self.path.is_none() {
            return Err("You have to provide the project's path when using `--yes` option.".into());
        }

        todo!();

        // Split the name into path components
        // such that dx new packages/app will create a directory called packages/app
        // let project_dir = self.path.unwrap_or_else(|| {
        //     let mut path = PathBuf::from(self.name.as_deref().unwrap());
        //
        //     // if path.is_relative() {
        //     //     path = std::env::current_dir().unwrap().join(path);
        //     // }
        //
        //     path = std::env::current_dir().unwrap().join(path);
        //
        //     // split the path into the parent and the name
        //     let parent = path.parent().unwrap();
        //     let name = path.file_name().unwrap();
        //     self.name = Some(name.to_str().unwrap().to_string());
        //
        //     // let get_parent_and_name = || -> Option<(&Path, String)> {
        //     //     let parent = path.parent()?;
        //     //     let name = path.file_name()?.to_str()?.to_string();
        //     //     Some((parent, name))
        //     // };
        //     // let (parent, name) = get_parent_and_name().unwrap();
        //     // self.name = Some(name);
        //
        //     // create the parent directory if it doesn't exist
        //     std::fs::create_dir_all(parent).unwrap();
        //
        //     // And then the "destination" is the parent directory
        //     parent.to_path_buf()
        // });

        // if self.name.is_none() {
        //     let dir_name = self
        //         .path
        //         .absolutize()?
        //         .to_path_buf()
        //         .file_name()
        //         .ok_or(Error::RuntimeError(
        //             "Current path does not include directory name".into(),
        //         ))?
        //         .to_str()
        //         .ok_or(Error::RuntimeError(
        //             "Current directory name is not a valid UTF-8 string".into(),
        //         ))?
        //         .to_string();
        //     self.name = Some(dir_name);
        // }

        let args = GenerateArgs {
            define: self.option,
            destination: self.path,
            // https://github.com/cargo-generate/cargo-generate/issues/1250
            init: true,
            name: self.name,
            silent: self.yes,
            template_path: TemplatePath {
                auto_path: Some(self.template),
                subfolder: self.subtemplate,
                ..Default::default()
            },
            ..Default::default()
        };

        let path = cargo_generate::generate(args)?;
        // let m = cargo_metadata::MetadataCommand::new()
        //     .current_dir(project_dir)
        //     .exec()
        //     .unwrap();
        // let root = m.workspace_root;
        // let name = m
        //     .packages
        //     .iter()
        //     .filter_map(|p| {
        //         if p.manifest_path.parent().unwrap() == root {
        //             Some(p.name.clone())
        //         } else {
        //             None
        //         }
        //     })
        //     .next()
        //     .expect("should be only 1 package");

        post_create(&path)
    }
}

/// Post-creation actions for newly setup crates.
// Also used by `init`.
pub(crate) fn post_create(path: &Path) -> Result<()> {
    let parent_dir = path.parent();
    let metadata = if parent_dir.is_none() {
        None
    } else {
        match cargo_metadata::MetadataCommand::new()
            .current_dir(parent_dir.unwrap())
            .exec()
        {
            Ok(v) => Some(v),
            // Only 1 error means that CWD isn't a cargo project.
            Err(cargo_metadata::Error::CargoMetadata { .. }) => None,
            Err(err) => return Err(Error::CargoMetadata(err)),
        }
    };

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
