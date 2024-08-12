#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub mod assets;
pub mod dx_build_info;
pub mod serve;
pub mod tools;
pub mod tracer;

pub mod cli;
pub use cli::*;

pub mod error;
pub use error::*;

pub(crate) mod builder;

mod dioxus_crate;
pub use dioxus_crate::*;

mod settings;
pub(crate) use settings::*;

pub(crate) mod metadata;

use anyhow::Context;
use clap::Parser;

use Commands::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let log_control = tracer::build_tracing();

    match args.action {
        Translate(opts) => opts
            .translate()
            .context(error_wrapper("Translation of HTML into RSX failed")),

        New(opts) => opts
            .create()
            .context(error_wrapper("Creating new project failed")),

        Init(opts) => opts
            .init()
            .context(error_wrapper("Initializing a new project failed")),

        Config(opts) => opts
            .config()
            .context(error_wrapper("Configuring new project failed")),

        Autoformat(opts) => opts
            .autoformat()
            .context(error_wrapper("Error autoformatting RSX")),

        Check(opts) => opts
            .check()
            .await
            .context(error_wrapper("Error checking RSX")),

        Link(opts) => opts
            .link()
            .context(error_wrapper("Error with linker passthrough")),

        Build(mut opts) => opts
            .run()
            .await
            .context(error_wrapper("Building project failed")),

        Clean(opts) => opts
            .clean()
            .context(error_wrapper("Cleaning project failed")),

        Serve(opts) => opts
            .serve(log_control)
            .await
            .context(error_wrapper("Serving project failed")),

        Bundle(opts) => opts
            .bundle()
            .await
            .context(error_wrapper("Bundling project failed")),
    }
}

/// Simplifies error messages that use the same pattern.
fn error_wrapper(message: &str) -> String {
    format!("🚫 {message}:")
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use std::fs::{create_dir_all, read_to_string};
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;
    use toml::Value;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn subcommand_init() -> Result<Command> {
        let mut command = Command::cargo_bin(env!("CARGO_BIN_NAME"))?;
        command
            .arg("init")
            .arg("--subtemplate")
            .arg("Fullstack")
            .arg("-o")
            .arg("styling=Vanilla")
            .arg("-o")
            .arg("router=false");
        Ok(command)
    }

    fn get_cargo_toml_path(project_path: &Path) -> PathBuf {
        project_path.join("Cargo.toml")
    }

    fn get_project_name(cargo_toml_path: &Path) -> Result<String> {
        Ok(toml::from_str::<Value>(&read_to_string(cargo_toml_path)?)?
            .get("package")
            .unwrap()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string())
    }

    #[test]
    fn test_subcommand_init_with_default_path() -> Result<()> {
        let project_dir = "dir";
        let project_name = project_dir;

        let temp_dir = tempdir()?;
        // Make current dir's name deterministic.
        let current_dir = temp_dir.path().join(project_dir);
        create_dir_all(&current_dir)?;
        let project_path = &current_dir;
        assert!(project_path.exists());

        subcommand_init()?
            .current_dir(&current_dir)
            .assert()
            .success();

        let cargo_toml_path = get_cargo_toml_path(project_path);
        assert!(cargo_toml_path.exists());
        assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
        Ok(())
    }

    #[test]
    fn test_subcommand_init_with_1_dir_path() -> Result<()> {
        let project_dir = "dir";
        let project_name = project_dir;

        let current_dir = tempdir()?;

        subcommand_init()?
            .arg(project_dir)
            .current_dir(&current_dir)
            .assert()
            .success();

        let project_path = current_dir.path().join(project_dir);
        let cargo_toml_path = get_cargo_toml_path(&project_path);
        assert!(project_path.exists());
        assert!(cargo_toml_path.exists());
        assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
        Ok(())
    }

    #[test]
    fn test_subcommand_init_with_2_dir_path() -> Result<()> {
        let project_dir = "a/b";
        let project_name = "b";

        let current_dir = tempdir()?;

        subcommand_init()?
            .arg(project_dir)
            .current_dir(&current_dir)
            .assert()
            .success();

        let project_path = current_dir.path().join(project_dir);
        let cargo_toml_path = get_cargo_toml_path(&project_path);
        assert!(project_path.exists());
        assert!(cargo_toml_path.exists());
        assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
        Ok(())
    }

    #[test]
    fn test_subcommand_init_with_default_path_and_custom_name() -> Result<()> {
        let project_dir = "dir";
        let project_name = "project";

        let temp_dir = tempdir()?;
        // Make current dir's name deterministic.
        let current_dir = temp_dir.path().join(project_dir);
        create_dir_all(&current_dir)?;
        let project_path = &current_dir;
        assert!(project_path.exists());

        subcommand_init()?
            .arg("--name")
            .arg(project_name)
            .current_dir(&current_dir)
            .assert()
            .success();

        let cargo_toml_path = get_cargo_toml_path(project_path);
        assert!(cargo_toml_path.exists());
        assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
        Ok(())
    }

    #[test]
    fn test_subcommand_init_with_1_dir_path_and_custom_name() -> Result<()> {
        let project_dir = "dir";
        let project_name = "project";

        let current_dir = tempdir()?;

        subcommand_init()?
            .arg(project_dir)
            .arg("--name")
            .arg(project_name)
            .current_dir(&current_dir)
            .assert()
            .success();

        let project_path = current_dir.path().join(project_dir);
        let cargo_toml_path = get_cargo_toml_path(&project_path);
        assert!(project_path.exists());
        assert!(cargo_toml_path.exists());
        assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
        Ok(())
    }

    #[test]
    fn test_subcommand_init_with_2_dir_path_and_custom_name() -> Result<()> {
        let project_dir = "a/b";
        let project_name = "project";

        let current_dir = tempdir()?;

        subcommand_init()?
            .arg(project_dir)
            .arg("--name")
            .arg(project_name)
            .current_dir(&current_dir)
            .assert()
            .success();

        let project_path = current_dir.path().join(project_dir);
        let cargo_toml_path = get_cargo_toml_path(&project_path);
        assert!(project_path.exists());
        assert!(cargo_toml_path.exists());
        assert_eq!(get_project_name(&cargo_toml_path)?, project_name);
        Ok(())
    }
}
