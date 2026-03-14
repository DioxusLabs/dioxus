use super::*;
use crate::TraceSrc;
use anyhow::{bail, Context};
use cargo_generate::{GenerateArgs, TemplatePath, Vcs};
use git2::ConfigLevel;
use std::{
    fs,
    panic::AssertUnwindSafe,
    path::Path,
    sync::{LazyLock, Mutex},
};
use tempfile::NamedTempFile;

static LIBGIT2_CONFIG_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) static DEFAULT_TEMPLATE: &str = "gh:dioxuslabs/dioxus-template";

#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "new")]
pub struct Create {
    /// Create a new Dioxus project at PATH
    pub path: PathBuf,

    /// Project name. Defaults to directory name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Template path
    #[clap(short, long)]
    pub template: Option<String>,

    /// Branch to select when using `template` from a git repository.
    /// Mutually exclusive with: `--revision`, `--tag`.
    #[clap(long, conflicts_with_all(["revision", "tag"]))]
    pub branch: Option<String>,

    /// A commit hash to select when using `template` from a git repository.
    /// Mutually exclusive with: `--branch`, `--tag`.
    #[clap(long, conflicts_with_all(["branch", "tag"]))]
    pub revision: Option<String>,

    /// Tag to select when using `template` from a git repository.
    /// Mutually exclusive with: `--branch`, `--revision`.
    #[clap(long, conflicts_with_all(["branch", "revision"]))]
    pub tag: Option<String>,

    /// Specify a sub-template within the template repository to be used as the actual template
    #[clap(long)]
    pub subtemplate: Option<String>,

    /// Pass `<option>=<value>` for the used template (e.g., `foo=bar`)
    #[clap(short, long)]
    pub option: Vec<String>,

    /// Skip user interaction by using the default values for the used template.
    /// Default values can be overridden with `--option`
    #[clap(short, long)]
    pub yes: bool,

    /// Specify the VCS used to initialize the generated template.
    /// Options: `git`, `none`.
    #[arg(long, value_parser)]
    pub vcs: Option<Vcs>,
}

impl Create {
    pub async fn create(mut self) -> Result<StructuredOutput> {
        // Project name defaults to directory name.
        if self.name.is_none() {
            self.name = Some(create::name_from_path(&self.path)?);
        }

        check_path(&self.path).await?;

        // Perform a connectivity check so we just don't it around doing nothing if there's a network error
        if self.template.is_none() {
            check_connectivity().await?;
        }

        // If no template is specified, use the default one and set the branch to the latest release.
        resolve_template_and_branch(&mut self.template, &mut self.branch);

        // cargo-generate requires the path to be created first.
        std::fs::create_dir_all(&self.path)?;

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
            vcs: self.vcs,
            template_path: TemplatePath {
                auto_path: self.template,
                branch: self.branch,
                revision: self.revision,
                subfolder: self.subtemplate,
                tag: self.tag,
                ..Default::default()
            },
            verbose: crate::logging::VERBOSITY
                .get()
                .map(|f| f.verbose)
                .unwrap_or(false),
            ..Default::default()
        };

        tracing::debug!(dx_src = ?TraceSrc::Dev, "Creating new project with args: {args:#?}");
        let path = cargo_generate_with_gitconfig_fallback(args)?;

        _ = post_create(&path, &self.vcs.unwrap_or(Vcs::Git));

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

/// Extracts the last directory name from the `path`.
pub(crate) fn name_from_path(path: &Path) -> Result<String> {
    use path_absolutize::Absolutize;

    Ok(path
        .absolutize()?
        .to_path_buf()
        .file_name()
        .context("Current path does not include directory name".to_string())?
        .to_str()
        .context("Current directory name is not a valid UTF-8 string".to_string())?
        .to_string())
}

/// Post-creation actions for newly setup crates.
pub(crate) fn post_create(path: &Path, vcs: &Vcs) -> Result<()> {
    let metadata = if let Some(parent_dir) = path.parent() {
        match cargo_metadata::MetadataCommand::new()
            .current_dir(parent_dir)
            .exec()
        {
            Ok(v) => Some(v),
            // Only 1 error means that CWD isn't a cargo project.
            Err(cargo_metadata::Error::CargoMetadata { .. }) => None,
            Err(err) => {
                anyhow::bail!("Couldn't retrieve cargo metadata: {:?}", err)
            }
        }
    } else {
        None
    };

    // 1. Add the new project to the workspace, if it exists.
    //    This must be executed first in order to run `cargo fmt` on the new project.
    let is_workspace = metadata.is_some();
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
            anyhow::anyhow!("failed to parse toml at {}: {}", toml_path.display(), e)
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

    // 5. Run git init
    if !is_workspace {
        vcs.initialize(path, Some("main"), true)?;
    }

    tracing::info!(dx_src = ?TraceSrc::Dev, "Generated project at {}\n\n`cd` to your project and run `dx serve` to start developing.\nMore information is available in the generated `README.md`.\n\nBuild cool things! ✌️", path.display());

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

/// Run cargo-generate, retrying with a sanitized gitconfig when the user's `url.*.insteadOf`
/// config rewrites a valid template URL into something libgit2/git2 can't fetch (eg `git@host:...`).
pub(crate) fn cargo_generate_with_gitconfig_fallback(args: GenerateArgs) -> Result<PathBuf> {
    match cargo_generate_generate(args.clone()) {
        Ok(path) => Ok(path),
        Err(err) => {
            let (should_retry, err) = match err {
                CargoGenerateFailure::Error(err) => {
                    (should_retry_with_sanitized_gitconfig(&err), err)
                }
                CargoGenerateFailure::Panic(payload) => {
                    let err = anyhow::anyhow!("cargo-generate panicked: {payload}");
                    (true, err)
                }
            };

            if !should_retry || args.gitconfig.is_some() {
                return Err(err);
            }

            tracing::debug!(
                dx_src = ?TraceSrc::Dev,
                "cargo-generate failed; retrying with a sanitized gitconfig to avoid url.insteadOf rewrites"
            );

            // Avoid racy, process-global libgit2 config mutation by serializing the fallback path.
            let _lock = LIBGIT2_CONFIG_LOCK
                .lock()
                .expect("LIBGIT2_CONFIG_LOCK mutex poisoned");

            let gitconfig = NamedTempFile::new()
                .context("failed to create temporary gitconfig for template generation")?;
            let mut retry_args = args;
            retry_args.gitconfig = Some(gitconfig.path().to_path_buf());

            // Keep the temp file alive for the duration of this call.
            let _libgit2_config_override = Libgit2ConfigOverride::new()?;

            cargo_generate_generate(retry_args).map_err(|retry_err| {
                let retry_err = match retry_err {
                    CargoGenerateFailure::Error(err) => err,
                    CargoGenerateFailure::Panic(payload) => {
                        anyhow::anyhow!("cargo-generate panicked: {payload}")
                    }
                };

                retry_err.context(
                    "Template generation failed. Note: dx retried with a sanitized gitconfig to avoid url.insteadOf rewrites.",
                )
            })
        }
    }
}

enum CargoGenerateFailure {
    Error(anyhow::Error),
    Panic(String),
}

fn cargo_generate_generate(
    args: GenerateArgs,
) -> std::result::Result<PathBuf, CargoGenerateFailure> {
    std::panic::catch_unwind(AssertUnwindSafe(|| cargo_generate::generate(args)))
        .map_err(|payload| CargoGenerateFailure::Panic(panic_payload_to_string(payload)))?
        .map_err(CargoGenerateFailure::Error)
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "<non-string panic payload>".to_string()
    }
}

struct Libgit2ConfigOverride {
    _tmp: tempfile::TempDir,
}

impl Libgit2ConfigOverride {
    fn new() -> Result<Self> {
        let tmp = tempfile::tempdir().context("failed to create temporary directory")?;
        fs::write(tmp.path().join(".gitconfig"), "")
            .context("failed to write sanitized .gitconfig")?;

        // These are process-global settings; see `LIBGIT2_CONFIG_LOCK`.
        unsafe {
            git2::opts::set_search_path(ConfigLevel::Global, tmp.path().to_string_lossy().as_ref())
                .context("failed to set libgit2 global config search path")?;

            if let Err(err) =
                git2::opts::set_search_path(ConfigLevel::XDG, tmp.path().to_string_lossy().as_ref())
            {
                let _ = git2::opts::reset_search_path(ConfigLevel::Global);
                return Err(err).context("failed to set libgit2 xdg config search path");
            }
        }

        Ok(Self { _tmp: tmp })
    }
}

impl Drop for Libgit2ConfigOverride {
    fn drop(&mut self) {
        unsafe {
            let _ = git2::opts::reset_search_path(ConfigLevel::Global);
            let _ = git2::opts::reset_search_path(ConfigLevel::XDG);
        }
    }
}

fn should_retry_with_sanitized_gitconfig(err: &anyhow::Error) -> bool {
    // `cargo-generate` wraps git clone errors with this context. The exact underlying libgit2
    // error varies (eg `class=Net`, `class=Ssh`, `class=Os` timeouts), so don't string-match on it.
    format!("{err:#}").contains("Please check if the Git user / repository exists.")
}

/// Check if the requested project can be created in the filesystem
pub(crate) async fn check_path(path: &std::path::PathBuf) -> Result<()> {
    match fs::metadata(path) {
        Ok(_metadata) => {
            bail!(
                "A file or directory with the given project name \"{}\" already exists.",
                path.to_string_lossy()
            )
        }
        Err(_err) => Ok(()),
    }
}

/// Perform a health check against github itself before we attempt to download any templates hosted
/// on github.
pub(crate) async fn check_connectivity() -> Result<()> {
    if crate::verbosity_or_default().offline {
        return Ok(());
    }

    use crate::styles::{GLOW_STYLE, LINK_STYLE};
    let client = reqwest::Client::new();
    for x in 0..=5 {
        tokio::select! {
            res = client.head("https://github.com/DioxusLabs/").header("User-Agent", "dioxus-cli").send() => {
                if res.is_ok() {
                    return Ok(());
                }
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            },
            _ = tokio::time::sleep(std::time::Duration::from_millis(if x == 1 { 500 } else { 2000 })) => {}
        }
        if x == 0 {
            eprintln!("{GLOW_STYLE}warning{GLOW_STYLE:#}: Waiting for {LINK_STYLE}https://github.com/dioxuslabs{LINK_STYLE:#}...")
        } else {
            eprintln!(
                "{GLOW_STYLE}warning{GLOW_STYLE:#}: ({x}/5) Taking a while, maybe your internet is down?"
            );
        }
    }

    bail!(
        "Error connecting to template repository. Try cloning the template manually or add `dioxus` to a `cargo new` project."
    )
}

#[cfg(test)]
mod gitconfig_tests {
    use super::*;
    use std::{
        env,
        ffi::OsString,
        process::{Command, Output},
    };

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(prev) => env::set_var(self.key, prev),
                None => env::remove_var(self.key),
            }
        }
    }

    fn set_env_var(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> EnvVarGuard {
        let previous = env::var_os(key);
        env::set_var(key, value);
        EnvVarGuard { key, previous }
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    fn run_git(current_dir: &Path, args: &[&str]) -> Output {
        Command::new("git")
            .args(args)
            .current_dir(current_dir)
            .output()
            .expect("git command failed to run")
    }

    fn assert_git_success(args: &[&str], output: &Output) {
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn file_url(path: &Path) -> String {
        format!("file:///{}", path.display().to_string().replace('\\', "/"))
    }

    fn write_template_repo(root: &Path) -> PathBuf {
        let template_dir = root.join("template");
        fs::create_dir_all(template_dir.join("src")).expect("create template src");

        fs::write(
            template_dir.join("Cargo.toml"),
            r#"[package]
name = "{{project-name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        )
        .expect("write template Cargo.toml");

        fs::write(
            template_dir.join("src/main.rs"),
            r#"fn main() {
    println!("hello");
}
"#,
        )
        .expect("write template src/main.rs");

        let init_args = ["init", "-b", "main"];
        let output = run_git(&template_dir, &init_args);
        assert_git_success(&init_args, &output);

        let add_args = ["add", "."];
        let output = run_git(&template_dir, &add_args);
        assert_git_success(&add_args, &output);

        let commit_args = [
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=test",
            "commit",
            "-m",
            "init template",
        ];
        let output = run_git(&template_dir, &commit_args);
        assert_git_success(&commit_args, &output);

        template_dir
    }

    #[test]
    fn cargo_generate_retries_with_sanitized_gitconfig() {
        let _env_lock = ENV_LOCK.lock().expect("ENV_LOCK mutex poisoned");
        if !git_available() {
            return;
        }

        let temp = tempfile::tempdir().expect("create temp dir");
        let template_dir = write_template_repo(temp.path());
        let template_url = file_url(&template_dir);

        let home_dir = temp.path().join("home");
        fs::create_dir_all(&home_dir).expect("create fake HOME dir");
        fs::write(
            home_dir.join(".gitconfig"),
            r#"[url "invalid://invalid-host/"]
    insteadOf = file:///
"#,
        )
        .expect("write .gitconfig");

        let _home = set_env_var("HOME", &home_dir);
        let _userprofile = set_env_var("USERPROFILE", &home_dir);

        let fail_dest = temp.path().join("out_fail").join("myapp");
        fs::create_dir_all(&fail_dest).expect("create fail destination");

        let args_fail = GenerateArgs {
            destination: Some(fail_dest),
            init: true,
            name: Some("myapp".to_string()),
            silent: true,
            vcs: Some(Vcs::None),
            template_path: TemplatePath {
                git: Some(template_url.clone()),
                branch: Some("main".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let err = cargo_generate::generate(args_fail).expect_err("expected cargo-generate to fail");
        assert!(
            super::should_retry_with_sanitized_gitconfig(&err),
            "expected retryable error, got:\n{err:#}"
        );

        let ok_dest = temp.path().join("out_ok").join("myapp");
        fs::create_dir_all(&ok_dest).expect("create ok destination");

        let args_ok = GenerateArgs {
            destination: Some(ok_dest.clone()),
            init: true,
            name: Some("myapp".to_string()),
            silent: true,
            vcs: Some(Vcs::None),
            template_path: TemplatePath {
                git: Some(template_url),
                branch: Some("main".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let out =
            cargo_generate_with_gitconfig_fallback(args_ok).expect("expected fallback to work");
        assert!(out.join("Cargo.toml").exists());
        assert!(out.join("src/main.rs").exists());
    }

    #[test]
    fn cargo_generate_retries_with_sanitized_gitconfig_on_class_os_error() {
        let _env_lock = ENV_LOCK.lock().expect("ENV_LOCK mutex poisoned");
        if !git_available() {
            return;
        }

        let temp = tempfile::tempdir().expect("create temp dir");
        let template_dir = write_template_repo(temp.path());
        let template_url = file_url(&template_dir);

        let home_dir = temp.path().join("home");
        fs::create_dir_all(&home_dir).expect("create fake HOME dir");
        fs::write(
            home_dir.join(".gitconfig"),
            r#"[url "file:///this/path/does/not/exist/"]
    insteadOf = file:///
"#,
        )
        .expect("write .gitconfig");

        let _home = set_env_var("HOME", &home_dir);
        let _userprofile = set_env_var("USERPROFILE", &home_dir);
        let _xdg_config_home = set_env_var("XDG_CONFIG_HOME", &home_dir);

        let fail_dest = temp.path().join("out_fail").join("myapp");
        fs::create_dir_all(&fail_dest).expect("create fail destination");

        let args_fail = GenerateArgs {
            destination: Some(fail_dest),
            init: true,
            name: Some("myapp".to_string()),
            silent: true,
            vcs: Some(Vcs::None),
            template_path: TemplatePath {
                git: Some(template_url.clone()),
                branch: Some("main".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let err = cargo_generate::generate(args_fail).expect_err("expected cargo-generate to fail");
        assert!(
            format!("{err:#}").contains("class=Os"),
            "expected a libgit2 OS error, got:\n{err:#}"
        );
        assert!(
            super::should_retry_with_sanitized_gitconfig(&err),
            "expected retryable error, got:\n{err:#}"
        );

        let ok_dest = temp.path().join("out_ok").join("myapp");
        fs::create_dir_all(&ok_dest).expect("create ok destination");

        let args_ok = GenerateArgs {
            destination: Some(ok_dest.clone()),
            init: true,
            name: Some("myapp".to_string()),
            silent: true,
            vcs: Some(Vcs::None),
            template_path: TemplatePath {
                git: Some(template_url),
                branch: Some("main".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let out =
            cargo_generate_with_gitconfig_fallback(args_ok).expect("expected fallback to work");
        assert!(out.join("Cargo.toml").exists());
        assert!(out.join("src/main.rs").exists());
    }
}

// todo: re-enable these tests with better parallelization
//
// #[cfg(test)]
// pub(crate) mod tests {
//     use escargot::{CargoBuild, CargoRun};
//     use std::sync::LazyLock;
//     use std::fs::{create_dir_all, read_to_string};
//     use std::path::{Path, PathBuf};
//     use std::process::Command;
//     use tempfile::tempdir;
//     use toml::Value;

//     static BINARY: LazyLock<CargoRun> = LazyLock::new(|| {
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
