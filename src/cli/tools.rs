use std::{
    fs::{create_dir_all, File},
    path::PathBuf,
};

use anyhow::Context;
use flate2::read::GzDecoder;
use futures::StreamExt;
use tar::Archive;
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq, Eq)]
pub enum Tool {
    Binaryen,
}

pub fn tool_list() -> Vec<&'static str> {
    vec!["binaryen"]
}

pub fn app_path() -> PathBuf {
    let data_local = dirs::data_local_dir().unwrap();
    let dioxus_dir = data_local.join("dioxus");
    if !dioxus_dir.is_dir() {
        create_dir_all(&dioxus_dir).unwrap();
    }
    dioxus_dir
}

pub fn temp_path() -> PathBuf {
    let app_path = app_path();
    let temp_path = app_path.join("temp");
    if !temp_path.is_dir() {
        create_dir_all(&temp_path).unwrap();
    }
    temp_path
}

pub fn tools_path() -> PathBuf {
    let app_path = app_path();
    let temp_path = app_path.join("tools");
    if !temp_path.is_dir() {
        create_dir_all(&temp_path).unwrap();
    }
    temp_path
}

#[allow(clippy::should_implement_trait)]
impl Tool {
    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            "binaryen" => Some(Self::Binaryen),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Binaryen => "binaryen",
        }
    }

    pub fn bin_path(&self) -> &str {
        match self {
            Self::Binaryen => "bin",
        }
    }

    pub fn target_platform(&self) -> &str {
        match self {
            Self::Binaryen => {
                if cfg!(target_os = "windows") {
                    "windows"
                } else if cfg!(target_os = "macos") {
                    "macos"
                } else if cfg!(target_os = "linux") {
                    "linux"
                } else {
                    panic!("unsupported platformm");
                }
            }
        }
    }

    pub fn download_url(&self) -> String {
        match self {
            Self::Binaryen => {
                format!(
                    "https://github.com/WebAssembly/binaryen/releases/download/version_105/binaryen-version_105-x86_64-{target}.tar.gz",
                    target = self.target_platform()
                )
            }
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            Self::Binaryen => "tar.gz",
        }
    }

    pub fn is_installed(&self) -> bool {
        tools_path().join(self.name()).is_dir()
    }

    pub fn temp_out_path(&self) -> PathBuf {
        temp_path().join(format!("{}-tool.tmp", self.name()))
    }

    pub async fn download_package(&self) -> anyhow::Result<PathBuf> {
        let download_url = self.download_url();
        let temp_out = self.temp_out_path();
        let mut file = tokio::fs::File::create(&temp_out)
            .await
            .context("failed creating temporary output file")?;

        let resp = reqwest::get(download_url).await.unwrap();

        let mut res_bytes = resp.bytes_stream();
        while let Some(chunk_res) = res_bytes.next().await {
            let chunk = chunk_res.context("error reading chunk from download")?;
            let _ = file.write(chunk.as_ref()).await;
        }

        Ok(temp_out)
    }

    pub async fn install_package(&self) -> anyhow::Result<()> {
        let temp_path = self.temp_out_path();
        let tool_path = tools_path();

        let dir_name = if self == &Tool::Binaryen {
            "binaryen-version_105"
        } else {
            ""
        };

        if self.extension() == "tar.gz" {
            let tar_gz = File::open(temp_path)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            archive.unpack(&tool_path)?;
            // println!("{:?} -> {:?}", tool_path.join(dir_name), tool_path.join(self.name()));
            std::fs::rename(
                tool_path.join(dir_name),
                tool_path.join(self.name()),
            )?;
        }

        Ok(())
    }
}
