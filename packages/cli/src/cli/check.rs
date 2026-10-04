//! Run linting against the user's codebase.
//!
//! For reference, the rustfmt main.rs file
//! <https://github.com/rust-lang/rustfmt/blob/master/src/bin/main.rs>

use super::*;
use crate::{AppBuilder, BuildId, BuildKind, BuildMode};
use anyhow::{Context, anyhow};
use futures_util::{StreamExt, stream::FuturesUnordered};
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
    pub(crate) async fn check(self) -> Result<StructuredOutput> {
        match self.file {
            // Default to checking the project through `cargo check` plus the syn lints, scoped to
            // the exact files rustc reported in its dep-info.
            None => {
                let BuildTargets { client, server } = self.build_args.into_targets().await?;

                let mut requests = vec![(client, BuildId::PRIMARY)];
                if let Some(server) = server {
                    if server.package != requests[0].0.package {
                        requests.push((server, BuildId::SECONDARY));
                    }
                }

                let mut files = vec![];
                for (mut req, build_id) in requests {
                    req.kind = BuildKind::Check;
                    tracing::info!("Checking {} [{}]...", req.package, req.triple);
                    let artifacts = AppBuilder::started(&req, BuildMode::Base, build_id)?
                        .finish_build()
                        .await?;

                    files.extend(
                        artifacts
                            .depinfo
                            .files
                            .iter()
                            .filter(|f| {
                                f.starts_with(req.crate_dir())
                                    && f.extension().is_some_and(|e| e == "rs")
                            })
                            .cloned(),
                    );
                }

                check_files_and_report(files).await?;
            }
            Some(file) => {
                check_files_and_report(vec![file])
                    .await
                    .context("error checking file")?;
            }
        }

        Ok(StructuredOutput::Success)
    }
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
                    .map(|contents| crate::check::check_file(_path, &contents))
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
        _ => Err(anyhow!("{total_issues} issues found.")),
    }
}

pub(crate) fn collect_rs_files(folder: &Path, files: &mut Vec<PathBuf>) {
    for entry in ignore::Walk::new(folder).flatten() {
        if entry.path().extension() == Some("rs".as_ref()) {
            files.push(entry.path().to_path_buf());
        }
    }
}
