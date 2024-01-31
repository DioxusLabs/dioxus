use futures_util::{stream::FuturesUnordered, StreamExt};
use std::{path::Path, process::exit};

use super::*;

// For reference, the rustfmt main.rs file
// https://github.com/rust-lang/rustfmt/blob/master/src/bin/main.rs

/// Check the Rust files in the project for issues.
#[derive(Clone, Debug, Parser)]
pub struct Check {
    /// Input file
    #[clap(short, long)]
    pub file: Option<PathBuf>,
}

impl Check {
    // Todo: check the entire crate
    pub async fn check(self) -> Result<()> {
        match self.file {
            // Default to checking the project
            None => {
                if let Err(e) = check_project_and_report().await {
                    eprintln!("error checking project: {}", e);
                    exit(1);
                }
            }
            Some(file) => {
                if let Err(e) = check_file_and_report(file).await {
                    eprintln!("failed to check file: {}", e);
                    exit(1);
                }
            }
        }

        Ok(())
    }
}

async fn check_file_and_report(path: PathBuf) -> Result<()> {
    check_files_and_report(vec![path]).await
}

/// Read every .rs file accessible when considering the .gitignore and check it
///
/// Runs using Tokio for multithreading, so it should be really really fast
///
/// Doesn't do mod-descending, so it will still try to check unreachable files. TODO.
async fn check_project_and_report() -> Result<()> {
    let crate_config = dioxus_cli_config::CrateConfig::new(None)?;

    let mut files_to_check = vec![];
    collect_rs_files(&crate_config.crate_dir, &mut files_to_check);
    check_files_and_report(files_to_check).await
}

/// Check a list of files and report the issues.
async fn check_files_and_report(files_to_check: Vec<PathBuf>) -> Result<()> {
    let issue_reports = files_to_check
        .into_iter()
        .filter(|file| file.components().all(|f| f.as_os_str() != "target"))
        .map(|path| async move {
            let _path = path.clone();
            let res = tokio::spawn(async move {
                tokio::fs::read_to_string(&_path)
                    .await
                    .map(|contents| dioxus_check::check_file(_path, &contents))
            })
            .await;

            if res.is_err() {
                eprintln!("error checking file: {}", path.display());
            }

            res
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    // remove error results which we've already printed
    let issue_reports = issue_reports
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<_>>();

    let total_issues = issue_reports.iter().map(|r| r.issues.len()).sum::<usize>();

    for report in issue_reports.into_iter() {
        if !report.issues.is_empty() {
            println!("{}", report);
        }
    }

    match total_issues {
        0 => println!("No issues found."),
        1 => println!("1 issue found."),
        _ => println!("{} issues found.", total_issues),
    }

    match total_issues {
        0 => exit(0),
        _ => exit(1),
    }
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
