use anyhow::anyhow;
use flate2::read::GzDecoder;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tar::Archive;
use tokio::{fs, process::Command};

pub(crate) struct WasmBindgen {
    version: String,
    input_path: PathBuf,
    out_dir: PathBuf,
    out_name: String,
    web: bool,
    debug: bool,
    keep_debug: bool,
    demangle: bool,
    remove_name_section: bool,
    remove_producers_section: bool,
}

impl WasmBindgen {
    pub async fn run(&self) -> anyhow::Result<()> {
        let binary_name = Self::installed_bin_name(&self.version);
        let path = Self::install_dir().await?.join(binary_name);

        Ok(())
    }

    /// Verify that the required wasm-bindgen version is installed.
    pub async fn verify_install(version: &str) -> anyhow::Result<bool> {
        let binary_name = Self::installed_bin_name(&version);
        let path = Self::install_dir().await?.join(binary_name);
        Ok(path.exists())
    }

    /// Install the specified wasm-bingen version.
    ///
    /// This will overwrite any existing wasm-bindgen binaries of the same version.
    ///
    /// This will attempt to install wasm-bindgen from:
    /// 1. Direct GitHub release download.
    /// 2. `cargo binstall` if installed.
    /// 3. Compile from source with `cargo install`.
    pub async fn install(version: &str) -> anyhow::Result<()> {
        if Self::install_github(version).await.is_ok() {
            return Ok(());
        }

        if Self::install_binstall(version).await.is_ok() {
            return Ok(());
        }

        if Self::install_cargo(version).await.is_ok() {
            return Ok(());
        }

        Err(anyhow!(
            "failed to install wasm-bindgen-cli@{version} from available sources"
        ))
    }

    /// Try installing wasm-bindgen-cli from GitHub.
    async fn install_github(version: &str) -> anyhow::Result<()> {
        let Some(url) = git_install_url(version) else {
            return Err(anyhow!(
                "no available GitHub binary for wasm-bindgen-cli@{version}"
            ));
        };

        // Download then extract wasm-bindgen-cli.
        let bytes = reqwest::get(url).await?.bytes().await?;
        let tar = GzDecoder::new(bytes.as_ref());
        let mut archive = Archive::new(tar);

        // Unpack the tar in the tmp dir
        let tmp_dir = Self::tmp_dir().await;
        archive.unpack(&tmp_dir);

        // Get the intermediate folder name from the tarball.
        // This varies by platform so we just read it.
        let dir_read = fs::read_dir(&tmp_dir).await?.next_entry().await?;
        let Some(dir_name) = dir_read
            .map(|entry| entry.file_name())
            .and_then(|name| name.to_str().map(|name_str| name_str.to_string()))
        else {
            return Err(anyhow!(
                "wasm-bindgen downloaded tar contained unexpected data"
            ));
        };

        // Get the final binary location.
        let installed_name = Self::installed_bin_name(&version);
        let install_dir = Self::install_dir().await?;
        let final_binary = install_dir.join(installed_name);

        // Move the install wasm-bindgen binary from tmp directory to it's new location.
        let containing_folder = tmp_dir.join(dir_name);
        let mut tmp_install_name = "wasm-bindgen";
        if cfg!(windows) {
            tmp_install_name = "wasm-bindgen.exe";
        }
        fs::copy(containing_folder.join(tmp_install_name), &final_binary).await?;

        Ok(())
    }

    /// Try installing wasm-bindgen-cli through cargo-binstall.
    async fn install_binstall(version: &str) -> anyhow::Result<()> {
        let package = Self::cargo_bin_name(version);
        let tmp_dir = Self::tmp_dir().await;

        // Run install command
        Command::new("cargo")
            .args([
                "binstall",
                &package,
                "--no-confirm",
                "--force",
                "--no-track",
                "--install-path",
                tmp_dir.to_str().expect("this should be utf8-compatable"),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Get the final binary location.
        let installed_name = Self::installed_bin_name(&version);
        let install_dir = Self::install_dir().await?;
        let final_binary = install_dir.join(installed_name);

        // Move the install wasm-bindgen binary from tmp directory to it's new location.
        let mut tmp_install_name = "wasm-bindgen";
        if cfg!(windows) {
            tmp_install_name = "wasm-bindgen.exe";
        }
        fs::copy(tmp_dir.join(tmp_install_name), &final_binary).await?;

        Ok(())
    }

    /// Try installing wasm-bindgen-cli from source using cargo install.
    async fn install_cargo(version: &str) -> anyhow::Result<()> {
        let package = Self::cargo_bin_name(version);
        let tmp_dir = std::env::temp_dir();

        // Run install command
        Command::new("cargo")
            .args([
                "install",
                &package,
                "--no-track",
                "--force",
                "--root",
                tmp_dir.to_str().expect("this should be utf8-compatable"),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Get the final binary location.
        let installed_name = Self::installed_bin_name(&version);
        let install_dir = Self::install_dir().await?;
        let final_binary = install_dir.join(installed_name);

        // Move the install wasm-bindgen binary from tmp directory to it's new location.
        let mut tmp_install_name = "wasm-bindgen";
        if cfg!(windows) {
            tmp_install_name = "wasm-bindgen.exe";
        }

        let tmp_installed_path = tmp_dir.join("bin").join(tmp_install_name);
        fs::copy(tmp_installed_path, &final_binary).await?;

        Ok(())
    }

    /// Get the installation directory for the wasm-bindgen executable.
    async fn install_dir() -> anyhow::Result<PathBuf> {
        let local = dirs::data_local_dir()
            .expect("user should be running on a compatible operating system");

        let bindgen_dir = local.join("dioxus/wasm-bindgen/");
        fs::create_dir_all(&bindgen_dir).await?;
        Ok(bindgen_dir)
    }

    /// Get a temp directory to install files into.
    async fn tmp_dir() -> PathBuf {
        let tmp_dir = std::env::temp_dir().join("dx-install-tmp");
        let _ = fs::remove_dir_all(&tmp_dir).await;
        tmp_dir
    }

    /// Get the name of a potentially installed wasm-bindgen binary.
    fn installed_bin_name(version: &str) -> String {
        let mut name = format!("wasm-bindgen-{version}");
        if cfg!(windows) {
            name = format!("{name}.exe");
        }
        name
    }

    /// Get the crates.io package name of wasm-bindgen-cli.
    fn cargo_bin_name(version: &str) -> String {
        format!("wasm-bindgen-cli@{version}")
    }
}

/// Get the GitHub installation URL for wasm-bindgen if it exists.
fn git_install_url(version: &str) -> Option<String> {
    let platform = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc.tar.gz"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-musl.tar.gz"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu.tar.gz"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin.tar.gz"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin.tar.gz"
    } else {
        return None;
    };

    Some(format!("https://github.com/rustwasm/wasm-bindgen/releases/download/{version}/wasm-bindgen-{version}-{platform}.tar.gz"))
}

/// A builder for WasmBindgen options.
pub(crate) struct WasmBindgenBuilder {
    version: String,
    input_path: PathBuf,
    out_dir: PathBuf,
    out_name: String,
    web: bool,
    debug: bool,
    keep_debug: bool,
    demangle: bool,
    remove_name_section: bool,
    remove_producers_section: bool,
}

impl WasmBindgenBuilder {
    pub fn new(version: String) -> Self {
        Self {
            version,
            input_path: PathBuf::new(),
            out_dir: PathBuf::new(),
            out_name: "bindgen".to_string(),
            web: true,
            debug: true,
            keep_debug: true,
            demangle: true,
            remove_name_section: false,
            remove_producers_section: false,
        }
    }

    pub fn build(self) -> WasmBindgen {
        WasmBindgen {
            version: self.version,
            input_path: self.input_path,
            out_dir: self.out_dir,
            out_name: self.out_name,
            web: self.web,
            debug: self.debug,
            keep_debug: self.keep_debug,
            demangle: self.demangle,
            remove_name_section: self.remove_name_section,
            remove_producers_section: self.remove_producers_section,
        }
    }

    pub fn input_path(self, input_path: &Path) -> Self {
        Self {
            input_path: input_path.to_path_buf(),
            ..self
        }
    }

    pub fn out_dir(self, out_dir: &Path) -> Self {
        Self {
            out_dir: out_dir.to_path_buf(),
            ..self
        }
    }

    pub fn out_name(self, out_name: String) -> Self {
        Self { out_name, ..self }
    }

    pub fn web(self, web: bool) -> Self {
        Self { web, ..self }
    }

    pub fn debug(self, debug: bool) -> Self {
        Self { debug, ..self }
    }

    pub fn keep_debug(self, keep_debug: bool) -> Self {
        Self { keep_debug, ..self }
    }

    pub fn demangle(self, demangle: bool) -> Self {
        Self { demangle, ..self }
    }

    pub fn remove_name_section(self, remove_name_section: bool) -> Self {
        Self {
            remove_name_section,
            ..self
        }
    }

    pub fn remove_producers_section(self, remove_producers_section: bool) -> Self {
        Self {
            remove_producers_section,
            ..self
        }
    }
}
