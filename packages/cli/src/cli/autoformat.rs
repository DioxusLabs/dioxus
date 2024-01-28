use dioxus_autofmt::{IndentOptions, IndentType};
use futures_util::{stream::FuturesUnordered, StreamExt};
use std::{fs, path::Path, process::exit};

use super::*;

// For reference, the rustfmt main.rs file
// https://github.com/rust-lang/rustfmt/blob/master/src/bin/main.rs

/// Format some rsx
#[derive(Clone, Debug, Parser)]
pub struct Autoformat {
    /// Run in 'check' mode. Exits with 0 if input is formatted correctly. Exits
    /// with 1 and prints a diff if formatting is required.
    #[clap(short, long)]
    pub check: bool,

    /// Input rsx (selection)
    #[clap(short, long)]
    pub raw: Option<String>,

    /// Input file
    #[clap(short, long)]
    pub file: Option<String>,

    /// Split attributes in lines or not
    #[clap(short, long, default_value = "false")]
    pub split_line_attributes: bool,
}

impl Autoformat {
    // Todo: autoformat the entire crate
    pub async fn autoformat(self) -> Result<()> {
        let Autoformat {
            check,
            raw,
            file,
            split_line_attributes,
            ..
        } = self;

        // Default to formatting the project
        if raw.is_none() && file.is_none() {
            if let Err(e) = autoformat_project(check, split_line_attributes).await {
                eprintln!("error formatting project: {}", e);
                exit(1);
            }
        }

        if let Some(raw) = raw {
            let indent = indentation_for(".", self.split_line_attributes)?;
            if let Some(inner) = dioxus_autofmt::fmt_block(&raw, 0, indent) {
                println!("{}", inner);
            } else {
                // exit process with error
                eprintln!("error formatting codeblock");
                exit(1);
            }
        }

        // Format single file
        if let Some(file) = file {
            refactor_file(file, split_line_attributes)?;
        }

        Ok(())
    }
}

fn refactor_file(file: String, split_line_attributes: bool) -> Result<(), Error> {
    let indent = indentation_for(".", split_line_attributes)?;
    let file_content = if file == "-" {
        let mut contents = String::new();
        std::io::stdin().read_to_string(&mut contents)?;
        Ok(contents)
    } else {
        fs::read_to_string(&file)
    };
    let Ok(s) = file_content else {
        eprintln!("failed to open file: {}", file_content.unwrap_err());
        exit(1);
    };
    let edits = dioxus_autofmt::fmt_file(&s, indent);
    let out = dioxus_autofmt::apply_formats(&s, edits);

    if file == "-" {
        print!("{}", out);
    } else if let Err(e) = fs::write(&file, out) {
        eprintln!("failed to write formatted content to file: {e}",);
    } else {
        println!("formatted {}", file);
    }

    Ok(())
}

fn get_project_files(config: &CrateConfig) -> Vec<PathBuf> {
    let mut files = vec![];

    let gitignore_path = config.crate_dir.join(".gitignore");
    if gitignore_path.is_file() {
        let gitigno = gitignore::File::new(gitignore_path.as_path()).unwrap();
        if let Ok(git_files) = gitigno.included_files() {
            let git_files = git_files
                .into_iter()
                .filter(|f| f.ends_with(".rs") && !is_target_dir(f));
            files.extend(git_files)
        };
    } else {
        collect_rs_files(&config.crate_dir, &mut files);
    }

    files
}

fn is_target_dir(file: &Path) -> bool {
    let stripped = if let Ok(cwd) = std::env::current_dir() {
        file.strip_prefix(cwd).unwrap_or(file)
    } else {
        file
    };
    if let Some(first) = stripped.components().next() {
        first.as_os_str() == "target"
    } else {
        false
    }
}

async fn format_file(
    path: impl AsRef<Path>,
    indent: IndentOptions,
) -> Result<usize, tokio::io::Error> {
    let contents = tokio::fs::read_to_string(&path).await?;

    let edits = dioxus_autofmt::fmt_file(&contents, indent);
    let len = edits.len();

    if !edits.is_empty() {
        let out = dioxus_autofmt::apply_formats(&contents, edits);
        tokio::fs::write(path, out).await?;
    }

    Ok(len)
}

/// Read every .rs file accessible when considering the .gitignore and try to format it
///
/// Runs using Tokio for multithreading, so it should be really really fast
///
/// Doesn't do mod-descending, so it will still try to format unreachable files. TODO.
async fn autoformat_project(check: bool, split_line_attributes: bool) -> Result<()> {
    let crate_config = dioxus_cli_config::CrateConfig::new(None)?;

    let files_to_format = get_project_files(&crate_config);

    if files_to_format.is_empty() {
        return Ok(());
    }

    let indent = indentation_for(&files_to_format[0], split_line_attributes)?;

    let counts = files_to_format
        .into_iter()
        .map(|path| async {
            let path_clone = path.clone();
            let res = tokio::spawn(format_file(path, indent.clone())).await;

            match res {
                Err(err) => {
                    eprintln!("error formatting file: {}\n{err}", path_clone.display());
                    None
                }
                Ok(Err(err)) => {
                    eprintln!("error formatting file: {}\n{err}", path_clone.display());
                    None
                }
                Ok(Ok(res)) => Some(res),
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    let files_formatted: usize = counts.into_iter().flatten().sum();

    if files_formatted > 0 && check {
        eprintln!("{} files needed formatting", files_formatted);
        exit(1);
    }

    Ok(())
}

fn indentation_for(
    file_or_dir: impl AsRef<Path>,
    split_line_attributes: bool,
) -> Result<IndentOptions> {
    let out = std::process::Command::new("cargo")
        .args(["fmt", "--", "--print-config", "current"])
        .arg(file_or_dir.as_ref())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output()?;
    if !out.status.success() {
        return Err(Error::CargoError("cargo fmt failed".into()));
    }

    let config = String::from_utf8_lossy(&out.stdout);

    let hard_tabs = config
        .lines()
        .find(|line| line.starts_with("hard_tabs "))
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| value.trim() == "true")
        .ok_or_else(|| {
            Error::RuntimeError("Could not find hard_tabs option in rustfmt config".into())
        })?;
    let tab_spaces = config
        .lines()
        .find(|line| line.starts_with("tab_spaces "))
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| value.trim().parse::<usize>())
        .ok_or_else(|| {
            Error::RuntimeError("Could not find tab_spaces option in rustfmt config".into())
        })?
        .map_err(|_| {
            Error::RuntimeError("Could not parse tab_spaces option in rustfmt config".into())
        })?;

    Ok(IndentOptions::new(
        if hard_tabs {
            IndentType::Tabs
        } else {
            IndentType::Spaces
        },
        tab_spaces,
        split_line_attributes,
    ))
}

fn collect_rs_files(folder: &impl AsRef<Path>, files: &mut Vec<PathBuf>) {
    if is_target_dir(folder.as_ref()) {
        return;
    }
    let Ok(folder) = folder.as_ref().read_dir() else {
        return;
    };
    // load the gitignore
    for entry in folder {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        }
        if let Some(ext) = path.extension() {
            if ext == "rs" && !is_target_dir(&path) {
                files.push(path);
            }
        }
    }
}

#[tokio::test]
async fn test_auto_fmt() {
    let test_rsx = r#"
                    //



                        div {}


                    //
                    //
                    //

                    "#
    .to_string();

    let fmt = Autoformat {
        check: false,
        raw: Some(test_rsx),
        file: None,
        split_line_attributes: false,
    };

    fmt.autoformat().await.unwrap();
}

/*#[test]
fn spawn_properly() {
    let out = Command::new("dioxus")
        .args([
            "fmt",
            "-f",
            r#"
//

rsx! {

    div {}
}

//
//
//

        "#,
        ])
        .output()
        .expect("failed to execute process");

    dbg!(out);
}*/
