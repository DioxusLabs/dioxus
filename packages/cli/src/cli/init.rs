use super::*;
use crate::cli::create::DEFAULT_TEMPLATE;
use cargo_generate::{GenerateArgs, TemplatePath};

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "init")]
pub struct Init {
    /// Create a new Dioxus project at PATH
    #[arg(default_value = ".")]
    path: PathBuf,

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

impl Init {
    pub fn init(mut self) -> Result<()> {
        // Project name defaults to directory name.
        if self.name.is_none() {
            self.name = Some(create::name_from_path(&self.path)?);
        }

        let args = GenerateArgs {
            define: self.option,
            destination: Some(self.path),
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
        create::restore_cursor_on_sigint();
        let path = cargo_generate::generate(args)?;
        create::post_create(&path)
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use std::fs::create_dir_all;
    use tempfile::tempdir;

    use super::create::tests::*;

    // Note: tests below (at least 6 of them) were written to mainly test
    // correctness of project's directory and its name, because previously it
    // was broken and tests bring a peace of mind. And also so that I don't have
    // to run my local hand-made tests every time.

    fn subcommand_init() -> Result<Command> {
        subcommand("init")
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
