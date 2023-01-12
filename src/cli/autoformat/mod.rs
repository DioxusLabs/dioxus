use futures::{stream::FuturesUnordered, StreamExt};
use std::process::exit;

use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Parser)]
pub struct Autoformat {
    /// Input rsx (selection)
    #[clap(short, long)]
    pub raw: Option<String>,

    /// Input file
    #[clap(short, long)]
    pub file: Option<String>,
}

impl Autoformat {
    // Todo: autoformat the entire crate
    pub async fn autoformat(self) -> Result<()> {
        // Default to formatting the project
        if self.raw.is_none() && self.file.is_none() {
            if let Err(e) = autoformat_project().await {
                eprintln!("error formatting project: {}", e);
                exit(1);
            }
        }

        if let Some(raw) = self.raw {
            if let Some(inner) = dioxus_autofmt::fmt_block(&raw, 0) {
                println!("{}", inner);
            } else {
                // exit process with error
                eprintln!("error formatting codeblock");
                exit(1);
            }
        }

        if let Some(file) = self.file {
            let edits = dioxus_autofmt::fmt_file(&file);
            let as_json = serde_json::to_string(&edits).unwrap();
            println!("{}", as_json);
        }

        Ok(())
    }
}

/// Read every .rs file accessible when considering the .gitignore and try to format it
///
/// Runs using Tokio for multithreading, so it should be really really fast
///
/// Doesn't do mod-descending, so it will still try to format unreachable files. TODO.
async fn autoformat_project() -> Result<()> {
    let crate_config = crate::CrateConfig::new()?;

    let mut files_to_format = vec![];
    collect_rs_files(&crate_config.crate_dir, &mut files_to_format);

    let counts = files_to_format
        .into_iter()
        .map(|path| {
            tokio::spawn(async move {
                let contents = tokio::fs::read_to_string(&path).await?;

                let edits = dioxus_autofmt::fmt_file(&contents);
                let len = edits.len();

                if !edits.is_empty() {
                    let out = dioxus_autofmt::apply_formats(&contents, edits);
                    tokio::fs::write(&path, out).await?;
                }

                Ok(len) as Result<usize, tokio::io::Error>
            })
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    let files_formatted: usize = counts
        .into_iter()
        .map(|f| match f {
            Ok(Ok(res)) => res,
            _ => 0,
        })
        .sum();

    println!("formatted {} blocks of rsx", files_formatted);

    Ok(())
}

fn collect_rs_files(folder: &PathBuf, files: &mut Vec<PathBuf>) {
    let Ok(folder) = folder.read_dir() else { return };

    // load the gitignore

    for entry in folder {
        let Ok(entry) = entry else { continue; };

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

#[test]
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
}
