use std::{
    fs::{create_dir_all, File},
    path::PathBuf,
    process::Command,
};

use anyhow::Context;
use flate2::read::GzDecoder;
use futures::StreamExt;
use tar::Archive;
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq, Eq)]
pub enum Tool {
    Binaryen,
    Sass,
}

pub fn tool_list() -> Vec<&'static str> {
    vec!["binaryen", "sass"]
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
    /// from str to tool enum
    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            "binaryen" => Some(Self::Binaryen),
            _ => None,
        }
    }

    /// get current tool name str
    pub fn name(&self) -> &str {
        match self {
            Self::Binaryen => "binaryen",
            Self::Sass => "sass",
        }
    }

    /// get tool bin dir path
    pub fn bin_path(&self) -> &str {
        match self {
            Self::Binaryen => "bin",
            Self::Sass => ".",
        }
    }

    /// get target platform
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
            Self::Sass => {
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

    /// get tool package download url
    pub fn download_url(&self) -> String {
        match self {
            Self::Binaryen => {
                format!(
                    "https://github.com/WebAssembly/binaryen/releases/download/version_105/binaryen-version_105-x86_64-{target}.tar.gz",
                    target = self.target_platform()
                )
            }
            Self::Sass => {
                format!(
                    "https://github.com/sass/dart-sass/releases/download/1.51.0/dart-sass-1.51.0-{target}-x64.tar.gz",
                    target = self.target_platform()
                )
            }
        }
    }

    /// get package extension name
    pub fn extension(&self) -> &str {
        match self {
            Self::Binaryen => "tar.gz",
            Self::Sass => {
                if cfg!(target_os = "windows") {
                    "zip"
                } else {
                    "tar.ge"
                }
            },
        }
    }

    /// check tool state
    pub fn is_installed(&self) -> bool {
        tools_path().join(self.name()).is_dir()
    }

    /// get download temp path
    pub fn temp_out_path(&self) -> PathBuf {
        temp_path().join(format!("{}-tool.tmp", self.name()))
    }

    /// start to download package
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

    /// start to install package
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
            std::fs::rename(tool_path.join(dir_name), tool_path.join(self.name()))?;
        }

        Ok(())
    }

    pub fn call(&self, command: &str, args: Vec<&str>) -> anyhow::Result<Vec<u8>> {
        let bin_path = tools_path().join(self.name()).join(self.bin_path());

        let command_file = match self {
            Tool::Binaryen => {
                if cfg!(target_os = "windows") {
                    format!("{}.exe", command)
                } else {
                    command.to_string()
                }
            }
            Tool::Sass => {
                command.to_string()
            }
        };

        if !bin_path.join(&command_file).is_file() {
            return Err(anyhow::anyhow!("Command file not found."));
        }

        let mut command = Command::new(bin_path.join(&command_file).to_str().unwrap());

        let output = command
            .args(&args[..])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()?;
        Ok(output.stdout)
    }
}
