use std::{
    fs::{create_dir_all, File},
    io::{ErrorKind, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Context;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use tar::Archive;
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq, Eq)]
pub enum Tool {
    Binaryen,
    Sass,
    Tailwind,
}

// pub fn tool_list() -> Vec<&'static str> {
//     vec!["binaryen", "sass", "tailwindcss"]
// }

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

pub fn clone_repo(dir: &Path, url: &str) -> anyhow::Result<()> {
    let target_dir = dir.parent().unwrap();
    let dir_name = dir.file_name().unwrap();

    let mut cmd = Command::new("git");
    let cmd = cmd.current_dir(target_dir);
    let res = cmd.arg("clone").arg(url).arg(dir_name).output();
    if let Err(err) = res {
        if ErrorKind::NotFound == err.kind() {
            log::warn!("Git program not found. Hint: Install git or check $PATH.");
            return Err(err.into());
        }
    }
    Ok(())
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
            "sass" => Some(Self::Sass),
            "tailwindcss" => Some(Self::Tailwind),
            _ => None,
        }
    }

    /// get current tool name str
    pub fn name(&self) -> &str {
        match self {
            Self::Binaryen => "binaryen",
            Self::Sass => "sass",
            Self::Tailwind => "tailwindcss",
        }
    }

    /// get tool bin dir path
    pub fn bin_path(&self) -> &str {
        match self {
            Self::Binaryen => "bin",
            Self::Sass => ".",
            Self::Tailwind => ".",
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
            Self::Tailwind => {
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

    /// get tool version
    pub fn tool_version(&self) -> &str {
        match self {
            Self::Binaryen => "version_105",
            Self::Sass => "1.51.0",
            Self::Tailwind => "v3.1.6",
        }
    }

    /// get tool package download url
    pub fn download_url(&self) -> String {
        match self {
            Self::Binaryen => {
                format!(
                    "https://github.com/WebAssembly/binaryen/releases/download/{version}/binaryen-{version}-x86_64-{target}.tar.gz",
                    version = self.tool_version(),
                    target = self.target_platform()
                )
            }
            Self::Sass => {
                format!(
                    "https://github.com/sass/dart-sass/releases/download/{version}/dart-sass-{version}-{target}-x64.{extension}",
                    version = self.tool_version(),
                    target = self.target_platform(),
                    extension = self.extension()
                )
            }
            Self::Tailwind => {
                let windows_extension = match self.target_platform() {
                    "windows" => ".exe",
                    _ => "",
                };
                format!(
                    "https://github.com/tailwindlabs/tailwindcss/releases/download/{version}/tailwindcss-{target}-x64{optional_ext}",
                    version = self.tool_version(),
                    target = self.target_platform(),
                    optional_ext = windows_extension
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
                    "tar.gz"
                }
            }
            Self::Tailwind => "bin",
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
        // log::info!("temp file path: {:?}", temp_out);
        Ok(temp_out)
    }

    /// start to install package
    pub async fn install_package(&self) -> anyhow::Result<()> {
        let temp_path = self.temp_out_path();
        let tool_path = tools_path();

        let dir_name = match self {
            Self::Binaryen => format!("binaryen-{}", self.tool_version()),
            Self::Sass => "dart-sass".to_string(),
            Self::Tailwind => self.name().to_string(),
        };

        if self.extension() == "tar.gz" {
            let tar_gz = File::open(temp_path)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            archive.unpack(&tool_path)?;
            std::fs::rename(tool_path.join(dir_name), tool_path.join(self.name()))?;
        } else if self.extension() == "zip" {
            // decompress the `zip` file
            extract_zip(&temp_path, &tool_path)?;
            std::fs::rename(tool_path.join(dir_name), tool_path.join(self.name()))?;
        } else if self.extension() == "bin" {
            let bin_path = match self.target_platform() {
                "windows" => tool_path.join(&dir_name).join(self.name()).join(".exe"),
                _ => tool_path.join(&dir_name).join(self.name()),
            };
            // Manualy creating tool directory because we directly download the binary via Github
            std::fs::create_dir(tool_path.join(dir_name))?;

            let mut final_file = std::fs::File::create(&bin_path)?;
            let mut temp_file = File::open(&temp_path)?;
            let mut content = Vec::new();

            temp_file.read_to_end(&mut content)?;
            final_file.write_all(&content)?;

            if self.target_platform() == "linux" {
                // This code does not update permissions idk why
                // let mut perms = final_file.metadata()?.permissions();
                // perms.set_mode(0o744);

                // Adding to the binary execution rights with "chmod"
                let mut command = Command::new("chmod");

                let _ = command
                    .args(vec!["+x", bin_path.to_str().unwrap()])
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .output()?;
            }

            std::fs::remove_file(&temp_path)?;
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
                if cfg!(target_os = "windows") {
                    format!("{}.bat", command)
                } else {
                    command.to_string()
                }
            }
            Tool::Tailwind => {
                if cfg!(target_os = "windows") {
                    format!("{}.exe", command)
                } else {
                    command.to_string()
                }
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

pub fn extract_zip(file: &Path, target: &Path) -> anyhow::Result<()> {
    let zip_file = std::fs::File::open(file)?;
    let mut zip = zip::ZipArchive::new(zip_file)?;

    if !target.exists() {
        std::fs::create_dir_all(target)?;
    }

    for i in 0..zip.len() {
        let mut zip_entry = zip.by_index(i)?;

        // check for dangerous paths
        // see https://docs.rs/zip/latest/zip/read/struct.ZipFile.html#warnings
        let Some(enclosed_name) = zip_entry.enclosed_name() else {
            return Err(anyhow::anyhow!(
                "Refusing to unpack zip entry with potentially dangerous path: zip={} entry={:?}",
                file.display(),
                zip_entry.name()
            ));
        };

        let output_path = target.join(enclosed_name);
        if zip_entry.is_dir() {
            std::fs::create_dir_all(output_path)?;
        } else {
            // create parent dirs if needed
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // extract file
            let mut target_file = if !output_path.exists() {
                std::fs::File::create(output_path)?
            } else {
                std::fs::File::open(output_path)?
            };
            let _num = std::io::copy(&mut zip_entry, &mut target_file)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_extract_zip() -> anyhow::Result<()> {
        let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests/fixtures/test.zip");
        let temp_dir = tempdir()?;
        let temp_path = temp_dir.path();

        extract_zip(path.as_path(), temp_path)?;

        let expected_files = vec!["file1.txt", "file2.txt", "dir/file3.txt"];
        for file in expected_files {
            let path = temp_path.join(file);
            assert!(path.exists(), "File not found: {:?}", path);
        }

        Ok(())
    }

    #[test]
    fn test_extract_zip_dangerous_path() -> anyhow::Result<()> {
        let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests/fixtures/dangerous.zip");
        let temp_dir = tempdir()?;
        let temp_path = temp_dir.path();

        let result = extract_zip(path.as_path(), temp_path);

        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Refusing to unpack zip entry with potentially dangerous path: zip="));
        assert!(err.to_string().contains("entry=\"/etc/passwd\""));

        Ok(())
    }
}
