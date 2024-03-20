//! Handles pre-compression for any static assets

use std::{ffi::OsString, path::PathBuf, pin::Pin};

use async_compression::tokio::bufread::GzipEncoder;
use futures_util::Future;
use tokio::task::JoinSet;

#[allow(unused)]
pub async fn pre_compress_files(directory: PathBuf) -> tokio::io::Result<()> {
    // print to stdin encoded gzip data
    pre_compress_dir(directory).await?;
    Ok(())
}

fn pre_compress_dir(
    path: PathBuf,
) -> Pin<Box<dyn Future<Output = tokio::io::Result<()>> + Send + Sync>> {
    Box::pin(async move {
        let mut entries = tokio::fs::read_dir(&path).await?;
        let mut set: JoinSet<tokio::io::Result<()>> = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            set.spawn(async move {
                if entry.file_type().await?.is_dir() {
                    if let Err(err) = pre_compress_dir(entry.path()).await {
                        tracing::error!(
                            "Failed to pre-compress directory {}: {}",
                            entry.path().display(),
                            err
                        );
                    }
                } else if let Err(err) = pre_compress_file(entry.path()).await {
                    tracing::error!(
                        "Failed to pre-compress static assets {}: {}",
                        entry.path().display(),
                        err
                    );
                }

                Ok(())
            });
        }
        while let Some(res) = set.join_next().await {
            res??;
        }
        Ok(())
    })
}

async fn pre_compress_file(path: PathBuf) -> tokio::io::Result<()> {
    let file = tokio::fs::File::open(&path).await?;
    let stream = tokio::io::BufReader::new(file);
    let mut encoder = GzipEncoder::new(stream);
    let new_extension = match path.extension() {
        Some(ext) => {
            if ext.to_string_lossy().to_lowercase().ends_with("gz") {
                return Ok(());
            }
            let mut ext = ext.to_os_string();
            ext.push(".gz");
            ext
        }
        None => OsString::from("gz"),
    };
    let output = path.with_extension(new_extension);
    let mut buffer = tokio::fs::File::create(&output).await?;
    tokio::io::copy(&mut encoder, &mut buffer).await?;
    Ok(())
}
