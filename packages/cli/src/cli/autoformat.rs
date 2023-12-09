use dioxus_autofmt::{IndentOptions, IndentType};
use futures::{stream::FuturesUnordered, StreamExt};
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
        // Default to formatting the project
        if self.raw.is_none() && self.file.is_none() {
            if let Err(e) = autoformat_project(self.check, self.split_line_attributes).await {
                eprintln!("error formatting project: {}", e);
                exit(1);
            }
        }

        if let Some(raw) = self.raw {
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
        if let Some(file) = self.file {
            let file_content;
            let indent;
            if file == "-" {
                indent = indentation_for(".", self.split_line_attributes)?;
                let mut contents = String::new();
                std::io::stdin().read_to_string(&mut contents)?;
                file_content = Ok(contents);
            } else {
                indent = indentation_for(".", self.split_line_attributes)?;
                file_content = fs::read_to_string(&file);
            };

            match file_content {
                Ok(s) => {
                    let edits = dioxus_autofmt::fmt_file(&s, indent);
                    let out = dioxus_autofmt::apply_formats(&s, edits);
                    if file == "-" {
                        print!("{}", out);
                    } else {
                        match fs::write(&file, out) {
                            Ok(_) => {
                                println!("formatted {}", file);
                            }
                            Err(e) => {
                                eprintln!("failed to write formatted content to file: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("failed to open file: {}", e);
                    exit(1);
                }
            }
        }

        Ok(())
    }
}

/// Read every .rs file accessible when considering the .gitignore and try to format it
///
/// Runs using Tokio for multithreading, so it should be really really fast
///
/// Doesn't do mod-descending, so it will still try to format unreachable files. TODO.
async fn autoformat_project(check: bool, split_line_attributes: bool) -> Result<()> {
    let crate_config = crate::CrateConfig::new(None)?;

    let mut files_to_format = vec![];
    collect_rs_files(&crate_config.crate_dir, &mut files_to_format);

    if files_to_format.is_empty() {
        return Ok(());
    }

    let indent = indentation_for(&files_to_format[0], split_line_attributes)?;

    let counts = files_to_format
        .into_iter()
        .filter(|file| {
            if file.components().any(|f| f.as_os_str() == "target") {
                return false;
            }

            true
        })
        .map(|path| async {
            let _path = path.clone();
            let _indent = indent.clone();
            let res = tokio::spawn(async move {
                let contents = tokio::fs::read_to_string(&path).await?;

                let edits = dioxus_autofmt::fmt_file(&contents, _indent.clone());
                let len = edits.len();

                if !edits.is_empty() {
                    let out = dioxus_autofmt::apply_formats(&contents, edits);
                    tokio::fs::write(&path, out).await?;
                }

                Ok(len) as Result<usize, tokio::io::Error>
            })
            .await;

            match res {
                Err(err) => {
                    eprintln!("error formatting file: {}\n{err}", _path.display());
                    None
                }
                Ok(Err(err)) => {
                    eprintln!("error formatting file: {}\n{err}", _path.display());
                    None
                }
                Ok(Ok(res)) => Some(res),
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    let files_formatted: usize = counts
        .into_iter()
        .map(|f| match f {
            Some(res) => res,
            _ => 0,
        })
        .sum();

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

fn collect_rs_files(folder: &Path, files: &mut Vec<PathBuf>) {
    let Ok(folder) = folder.read_dir() else {
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
            if ext == "rs" {
                files.push(path);
            }
        }
    }
}

#[tokio::test]
async fn test_auto_fmt() {
    let test_rsx = r#"
                    //

                    rsx! {

                        div {}
                    }

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
