use std::{fs::create_dir_all, path::PathBuf};

use anyhow::Context;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

pub enum Tool {
    WasmOpt,
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

impl Tool {
    pub fn name(&self) -> &str {
        match self {
            Self::WasmOpt => "wasm-opt",
        }
    }

    pub fn bin_path(&self) -> &str {
        if cfg!(target_os = "windows") {
            match self {
                Self::WasmOpt => "bin/wasm-opt.exe",
            }
        } else {
            match self {
                Self::WasmOpt => "bin/wasm-opt",
            }
        }
    }

    pub fn target_platform(&self) -> &str {
        match self {
            Self::WasmOpt => {
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
            Self::WasmOpt => {
                format!(
                    "https://github.com/WebAssembly/binaryen/releases/download/version_105/binaryen-version_105-x86_64-{target}.tar.gz",
                    target = self.target_platform()
                )
            }
        }
    }

    pub async fn download_package(&self) -> anyhow::Result<PathBuf> {
        let temp_dir = temp_path();
        let download_url = self.download_url();

        let temp_out = temp_dir.join(format!("{}-tool.tmp", self.name()));
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
}
