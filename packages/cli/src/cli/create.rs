use super::*;
use crate::TraceSrc;
use cargo_generate::{GenerateArgs, TemplatePath};
use std::path::Path;

pub(crate) static DEFAULT_TEMPLATE: &str = "gh:dioxuslabs/dioxus-template";

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "new")]
pub struct Create {
    /// Create a new Dioxus project at PATH
    path: PathBuf,

    /// Project name. Defaults to directory name
    #[arg(short, long)]
    name: Option<String>,

    /// Template path
    #[clap(short, long)]
    template: Option<String>,

    /// Branch to select when using `template` from a git repository.
    /// Mutually exclusive with: `--revision`, `--tag`.
    #[clap(long, conflicts_with_all(["revision", "tag"]))]
    branch: Option<String>,

    /// A commit hash to select when using `template` from a git repository.
    /// Mutually exclusive with: `--branch`, `--tag`.
    #[clap(long, conflicts_with_all(["branch", "tag"]))]
    revision: Option<String>,

    /// Tag to select when using `template` from a git repository.
    /// Mutually exclusive with: `--branch`, `--revision`.
    #[clap(long, conflicts_with_all(["branch", "revision"]))]
    tag: Option<String>,

    /// Specify a sub-template within the template repository to be used as the actual template
    #[clap(long)]
    subtemplate: Option<String>,

    /// Pass <option>=<value> for the used template (e.g., `foo=bar`)
    #[clap(short, long)]
    option: Vec<String>,

    /// Skip user interaction by using the default values for the used template.
    /// Default values can be overridden with `--option`
    #[clap(short, long)]
    yes: bool,
}

impl Create {
    pub fn create(mut self) -> Result<StructuredOutput> {
        // Project name defaults to directory name.
        if self.name.is_none() {
            self.name = Some(create::name_from_path(&self.path)?);
        }

        // If no template is specified, use the default one and set the branch to the latest release.
        resolve_template_and_branch(&mut self.template, &mut self.branch);

        let args = GenerateArgs {
            define: self.option,
            destination: Some(self.path),
            // NOTE: destination without init means base_dir + name, with —
            // means dest_dir. So use `init: true` and always handle
            // the dest_dir manually and carefully.
            // Cargo never adds name to the path. Name is solely for project name.
            // https://github.com/cargo-generate/cargo-generate/issues/1250
            init: true,
            name: self.name,
            silent: self.yes,
            template_path: TemplatePath {
                auto_path: self.template,
                branch: self.branch,
                revision: self.revision,
                subfolder: self.subtemplate,
                tag: self.tag,
                ..Default::default()
            },
            ..Default::default()
        };
        restore_cursor_on_sigint();
        let path = cargo_generate::generate(args)?;
        _ = post_create(&path);
        Ok(StructuredOutput::Success)
    }
}

/// If no template is specified, use the default one and set the branch to the latest release.
///
/// Allows us to version templates under the v0.5/v0.6 scheme on the templates repo.
pub(crate) fn resolve_template_and_branch(
    template: &mut Option<String>,
    branch: &mut Option<String>,
) {
    if template.is_none() {
        use crate::dx_build_info::{PKG_VERSION_MAJOR, PKG_VERSION_MINOR};
        *template = Some(DEFAULT_TEMPLATE.to_string());

        if branch.is_none() {
            *branch = Some(format!("v{PKG_VERSION_MAJOR}.{PKG_VERSION_MINOR}"));
        }
    };
}

/// Prevent hidden cursor if Ctrl+C is pressed when interacting
/// with cargo-generate's prompts.
///
/// See https://github.com/DioxusLabs/dioxus/pull/2603.
pub(crate) fn restore_cursor_on_sigint() {
    ctrlc::set_handler(move || {
        if let Err(err) = console::Term::stdout().show_cursor() {
            eprintln!("Error showing the cursor again: {err}");
        }
        std::process::exit(1); // Ideally should mimic the INT signal.
    })
    .expect("ctrlc::set_handler");
}

/// Extracts the last directory name from the `path`.
pub(crate) fn name_from_path(path: &Path) -> Result<String> {
    use path_absolutize::Absolutize;

    Ok(path
        .absolutize()?
        .to_path_buf()
        .file_name()
        .ok_or("Current path does not include directory name".to_string())?
        .to_str()
        .ok_or("Current directory name is not a valid UTF-8 string".to_string())?
        .to_string())
}

/// Post-creation actions for newly setup crates.
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
            Err(err) => {
                return Err(Error::Other(anyhow::anyhow!(
                    "Couldn't retrieve cargo metadata: {:?}",
                    err
                )));
            }
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
        tracing::error!(dx_src = ?TraceSrc::Dev, "cargo fmt failed");
        tracing::error!(dx_src = ?TraceSrc::Build, "stdout: {}", String::from_utf8_lossy(&output.stdout));
        tracing::error!(dx_src = ?TraceSrc::Build, "stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    // 3. Format the `Cargo.toml` and `Dioxus.toml` files.
    let toml_paths = [path.join("Cargo.toml"), path.join("Dioxus.toml")];
    for toml_path in &toml_paths {
        let Ok(toml) = std::fs::read_to_string(toml_path) else {
            continue;
        };

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

    tracing::info!(dx_src = ?TraceSrc::Dev, "Generated project at {}\n\n`cd` to your project and run `dx serve` to start developing.\nIf using Tailwind, make sure to run the Tailwind CLI.\nMore information is available in the generated `README.md`.\n\nBuild cool things! ✌️", path.display());

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

// todo: re-enable these tests with better parallelization
//
// #[cfg(test)]
// pub(crate) mod tests {
//     use escargot::{CargoBuild, CargoRun};
//     use once_cell::sync::Lazy;
//     use std::fs::{create_dir_all, read_to_string};
//     use std::path::{Path, PathBuf};
//     use std::process::Command;
//     use tempfile::tempdir;
//     use toml::Value;

//     static BINARY: Lazy<CargoRun> = Lazy::new(|| {
//         CargoBuild::new()
//             .bin(env!("CARGO_BIN_NAME"))
//             .current_release()
//             .run()
//             .expect("Couldn't build the binary for tests.")
//     });

//     // Note: tests below (at least 6 of them) were written to mainly test
//     // correctness of project's directory and its name, because previously it
//     // was broken and tests bring a peace of mind. And also so that I don't have
//     // to run my local hand-made tests every time.

//     pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

//     pub(crate) fn subcommand(name: &str) -> Command {
//         let mut command = BINARY.command();
//         command.arg(name).arg("--yes"); // Skip any questions by choosing default answers.
//         command
//     }

//     pub(crate) fn get_cargo_toml_path(project_path: &Path) -> PathBuf {
//         project_path.join("Cargo.toml")
//     }

//     pub(crate) fn get_project_name(cargo_toml_path: &Path) -> Result<String> {
//         Ok(toml::from_str::<Value>(&read_to_string(cargo_toml_path)?)?
//             .get("package")
//             .unwrap()
//             .get("name")
//             .unwrap()
//             .as_str()
//             .unwrap()
//             .to_string())
//     }

//     fn subcommand_new() -> Command {
//         subcommand("new")
//     }

//     #[test]
//     fn test_subcommand_new_with_dot_path() -> Result<()> {
//         let project_dir = "dir";
//         let project_name = project_dir;

//         let temp_dir = tempdir()?;
//         // Make current dir's name deterministic.
//         let current_dir = temp_dir.path().join(project_dir);
//         create_dir_all(&current_dir)?;
//         let project_path = &current_dir;
//         assert!(project_path.exists());

//         assert!(subcommand_new()
//             .arg(".")
//             .current_dir(&current_dir)
//             .status()
//             .is_ok());

//         let cargo_toml_path = get_cargo_toml_path(project_path);
//         assert!(cargo_toml_path.exists());
//         assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
//         Ok(())
//     }

//     #[test]
//     fn test_subcommand_new_with_1_dir_path() -> Result<()> {
//         let project_dir = "dir";
//         let project_name = project_dir;

//         let current_dir = tempdir()?;

//         assert!(subcommand_new()
//             .arg(project_dir)
//             .current_dir(&current_dir)
//             .status()
//             .is_ok());

//         let project_path = current_dir.path().join(project_dir);
//         let cargo_toml_path = get_cargo_toml_path(&project_path);
//         assert!(project_path.exists());
//         assert!(cargo_toml_path.exists());
//         assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
//         Ok(())
//     }

//     #[test]
//     fn test_subcommand_new_with_2_dir_path() -> Result<()> {
//         let project_dir = "a/b";
//         let project_name = "b";

//         let current_dir = tempdir()?;

//         assert!(subcommand_new()
//             .arg(project_dir)
//             .current_dir(&current_dir)
//             .status()
//             .is_ok());

//         let project_path = current_dir.path().join(project_dir);
//         let cargo_toml_path = get_cargo_toml_path(&project_path);
//         assert!(project_path.exists());
//         assert!(cargo_toml_path.exists());
//         assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
//         Ok(())
//     }

//     #[test]
//     fn test_subcommand_new_with_dot_path_and_custom_name() -> Result<()> {
//         let project_dir = "dir";
//         let project_name = "project";

//         let temp_dir = tempdir()?;
//         // Make current dir's name deterministic.
//         let current_dir = temp_dir.path().join(project_dir);
//         create_dir_all(&current_dir)?;
//         let project_path = &current_dir;
//         assert!(project_path.exists());

//         assert!(subcommand_new()
//             .arg("--name")
//             .arg(project_name)
//             .arg(".")
//             .current_dir(&current_dir)
//             .status()
//             .is_ok());

//         let cargo_toml_path = get_cargo_toml_path(project_path);
//         assert!(cargo_toml_path.exists());
//         assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
//         Ok(())
//     }

//     #[test]
//     fn test_subcommand_new_with_1_dir_path_and_custom_name() -> Result<()> {
//         let project_dir = "dir";
//         let project_name = "project";

//         let current_dir = tempdir()?;

//         assert!(subcommand_new()
//             .arg(project_dir)
//             .arg("--name")
//             .arg(project_name)
//             .current_dir(&current_dir)
//             .status()
//             .is_ok());

//         let project_path = current_dir.path().join(project_dir);
//         let cargo_toml_path = get_cargo_toml_path(&project_path);
//         assert!(project_path.exists());
//         assert!(cargo_toml_path.exists());
//         assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
//         Ok(())
//     }

//     #[test]
//     fn test_subcommand_new_with_2_dir_path_and_custom_name() -> Result<()> {
//         let project_dir = "a/b";
//         let project_name = "project";

//         let current_dir = tempdir()?;

//         assert!(subcommand_new()
//             .arg(project_dir)
//             .arg("--name")
//             .arg(project_name)
//             .current_dir(&current_dir)
//             .status()
//             .is_ok());

//         let project_path = current_dir.path().join(project_dir);
//         let cargo_toml_path = get_cargo_toml_path(&project_path);
//         assert!(project_path.exists());
//         assert!(cargo_toml_path.exists());
//         assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
//         Ok(())
//     }
// }
