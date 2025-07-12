//! Run linting against the user's codebase.
//!
//! For reference, the rustfmt main.rs file
//! <https://github.com/rust-lang/rustfmt/blob/master/src/bin/main.rs>

use super::*;
use crate::BuildRequest;
use anyhow::{anyhow, Context};
use futures_util::{stream::FuturesUnordered, StreamExt};
use std::path::Path;

/// Check the Rust files in the project for issues.
#[derive(Clone, Debug, Parser)]
pub(crate) struct Check {
    /// Input file
    #[clap(short, long)]
    pub(crate) file: Option<PathBuf>,

    /// Information about the target to check
    #[clap(flatten)]
    pub(crate) build_args: CommandWithPlatformOverrides<BuildArgs>,
}

impl Check {
    // Todo: check the entire crate
    pub(crate) async fn check(self) -> Result<StructuredOutput> {
        let BuildTargets { client, server } = self.build_args.into_targets().await?;

        match self.file {
            // Default to checking the project
            None => {
                check_project_and_report(&client)
                    .await
                    .context("error checking project")?;

                if let Some(server) = server {
                    if server.package != client.package {
                        check_project_and_report(&server)
                            .await
                            .context("error checking project")?;
                    }
                }
            }
            Some(file) => {
                check_file_and_report(file)
                    .await
                    .context("error checking file")?;
            }
        }

        Ok(StructuredOutput::Success)
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
async fn check_project_and_report(build: &BuildRequest) -> Result<()> {
    let dioxus_crate = build
        .workspace
        .find_main_package(Some(build.package.clone()))?;
    let dioxus_crate = &build.workspace.krates[dioxus_crate];
    let mut files_to_check = vec![];
    collect_rs_files(
        dioxus_crate.manifest_path.parent().unwrap().as_std_path(),
        &mut files_to_check,
    );
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
                tracing::error!("error checking file: {}", path.display());
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
            tracing::info!("{}", report);
        }
    }

    match total_issues {
        0 => {
            tracing::info!("No issues found.");
            Ok(())
        }
        1 => Err(anyhow!("1 issue found.")),
        _ => Err(anyhow!("{} issues found.", total_issues)),
    }
}

pub(crate) fn collect_rs_files(folder: &Path, files: &mut Vec<PathBuf>) {
    for entry in ignore::Walk::new(folder).flatten() {
        if entry.path().extension() == Some("rs".as_ref()) {
            files.push(entry.path().to_path_buf());
        }
    }
}
