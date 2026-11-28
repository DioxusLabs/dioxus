use crate::config::WasmOptLevel;
use crate::{CliSettings, Result, WasmOptConfig, Workspace};
use anyhow::{Context, anyhow, bail};
use flate2::read::GzDecoder;
use std::path::{Path, PathBuf};
use tar::Archive;
use tempfile::NamedTempFile;

/// Pinned binaryen version (contains wasm-opt).
const BINARYEN_VERSION: &str = "129";

/// Write these wasm bytes with a particular set of optimizations
pub async fn write_wasm(bytes: &[u8], output_path: &Path, cfg: &WasmOptConfig) -> Result<()> {
    std::fs::write(output_path, bytes)?;
    optimize(output_path, output_path, cfg).await?;
    Ok(())
}

pub async fn optimize(input_path: &Path, output_path: &Path, cfg: &WasmOptConfig) -> Result<()> {
    let wasm_opt = WasmOpt::new(input_path, output_path, cfg)
        .await
        .context("Failed to create wasm-opt instance")?;
    wasm_opt
        .optimize()
        .await
        .context("Failed to run wasm-opt")?;

    Ok(())
}

struct WasmOpt {
    path: PathBuf,
    input_path: PathBuf,
    temporary_output_path: NamedTempFile,
    output_path: PathBuf,
    cfg: WasmOptConfig,
}

impl WasmOpt {
    pub async fn new(
        input_path: &Path,
        output_path: &Path,
        cfg: &WasmOptConfig,
    ) -> anyhow::Result<Self> {
        let path = get_binary_path().await?;
        Ok(Self {
            path,
            input_path: input_path.to_path_buf(),
            temporary_output_path: tempfile::NamedTempFile::new()?,
            output_path: output_path.to_path_buf(),
            cfg: cfg.clone(),
        })
    }

    /// Create the command to run wasm-opt
    fn build_command(&self) -> tokio::process::Command {
        // defaults needed by wasm-opt.
        // wasm is a moving target, and we add these by default since they progressively get enabled by default.
        let mut args = vec![
            "--enable-reference-types",
            "--enable-bulk-memory",
            "--enable-mutable-globals",
            "--enable-nontrapping-float-to-int",
            "--enable-threads",
        ];

        if self.cfg.memory_packing {
            // needed for our current approach to bundle splitting to work properly
            // todo(jon): emit the main module's data section in chunks instead of all at once
            args.push("--memory-packing");
        }

        if !self.cfg.debug {
            args.push("--strip-debug");
        } else {
            args.push("--debuginfo");
        }

        for extra in &self.cfg.extra_features {
            args.push(extra);
        }

        let level = match self.cfg.level {
            WasmOptLevel::Z => "-Oz",
            WasmOptLevel::S => "-Os",
            WasmOptLevel::Zero => "-O0",
            WasmOptLevel::One => "-O1",
            WasmOptLevel::Two => "-O2",
            WasmOptLevel::Three => "-O3",
            WasmOptLevel::Four => "-O4",
        };

        tracing::debug!(
            "Running wasm-opt: {} {} {} -o {} {}",
            self.path.to_string_lossy(),
            self.input_path.to_string_lossy(),
            level,
            self.temporary_output_path.path().to_string_lossy(),
            args.join(" ")
        );
        let mut command = tokio::process::Command::new(&self.path);
        command
            .arg(&self.input_path)
            .arg(level)
            .arg("-o")
            .arg(self.temporary_output_path.path())
            .args(args);
        command
    }

    pub async fn optimize(&self) -> Result<()> {
        self.run(self.build_command()).await
    }

    async fn run(&self, mut command: tokio::process::Command) -> Result<()> {
        let res = command.output().await?;

        if !res.status.success() {
            let message = format!(
                "wasm-opt failed with status {}\nstderr: {}\nstdout: {}",
                res.status,
                String::from_utf8_lossy(&res.stderr),
                String::from_utf8_lossy(&res.stdout)
            );
            tracing::error!(
                telemetry = %serde_json::json!({ "event": "wasm_opt_failed" }),
                "{message}"
            );
            // A failed optimizer may leave partial output. Preserve the original wasm.
            bail!(message);
        }

        std::fs::copy(self.temporary_output_path.path(), &self.output_path).with_context(|| {
            format!(
                "Failed to write optimized wasm to {}",
                self.output_path.display()
            )
        })?;

        Ok(())
    }
}

// Find the URL for the latest binaryen release that contains wasm-opt
async fn find_latest_wasm_opt_download_url() -> anyhow::Result<String> {
    // Find the platform identifier based on the current OS and architecture
    // hardcoded for now to get around github api rate limits
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return Ok("https://github.com/WebAssembly/binaryen/releases/download/version_127/binaryen-version_127-x86_64-windows.tar.gz".to_string());
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        return Ok("https://github.com/WebAssembly/binaryen/releases/download/version_127/binaryen-version_127-arm64-windows.tar.gz".to_string());
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return Ok("https://github.com/WebAssembly/binaryen/releases/download/version_127/binaryen-version_127-x86_64-linux.tar.gz".to_string());
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        return Ok("https://github.com/WebAssembly/binaryen/releases/download/version_127/binaryen-version_127-aarch64-linux.tar.gz".to_string());
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        return Ok("https://github.com/WebAssembly/binaryen/releases/download/version_127/binaryen-version_127-x86_64-macos.tar.gz".to_string());
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Ok("https://github.com/WebAssembly/binaryen/releases/download/version_127/binaryen-version_127-arm64-macos.tar.gz".to_string());
    };

    let url = "https://api.github.com/repos/WebAssembly/binaryen/releases/latest";
    let client = crate::http_client::client();
    let response = client
        .get(url)
        .header("User-Agent", "dioxus-cli")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    tracing::trace!("Response from GitHub: {:#?}", response);

    let assets = response
        .get("assets")
        .and_then(|assets| assets.as_array())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse assets"))?;

    // Find the platform identifier based on the current OS and architecture
    let platform = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-windows"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "arm64-windows"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-linux"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-linux"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-macos"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "arm64-macos"
    } else {
        bail!(
            "Unknown platform for wasm-opt installation. Please install wasm-opt manually from https://github.com/WebAssembly/binaryen/releases and add it to your PATH."
        );
    };

    // Find the first asset with a name that contains the platform string
    let asset = assets
        .iter()
        .find(|asset| {
            asset
                .get("name")
                .and_then(|name| name.as_str())
                .is_some_and(|name| name.contains(platform) && !name.ends_with("sha256"))
        })
        .with_context(|| anyhow!(
            "No suitable wasm-opt binary found for platform: {}. Please install wasm-opt manually from https://github.com/WebAssembly/binaryen/releases and add it to your PATH.",
            platform
        ))?;

    // Extract the download URL from the asset
    let download_url = asset
        .get("browser_download_url")
        .and_then(|url| url.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get download URL for wasm-opt"))?;

    Ok(download_url.to_string())
}

/// Get the path to the wasm-opt binary, downloading it if necessary
pub async fn get_binary_path() -> anyhow::Result<PathBuf> {
    let install_dir = install_dir();
    let install_path = installed_bin_path(&install_dir);

    if install_path.exists() {
        return Ok(install_path);
    }

    if CliSettings::prefer_no_downloads() {
        if let Ok(existing) = which::which("wasm-opt") {
            return Ok(existing);
        } else {
            return Err(anyhow!("Missing wasm-opt"));
        }
    }

    tracing::info!("Installing wasm-opt");
    install_github(&install_dir).await?;
    tracing::info!("wasm-opt installed from Github");

    Ok(install_path)
}

pub fn installed_location() -> Option<PathBuf> {
    let install_dir = install_dir();
    let install_path = installed_bin_path(&install_dir);

    if install_path.exists() {
        return Some(install_path);
    }

    if CliSettings::prefer_no_downloads() {
        if let Ok(existing) = which::which("wasm-opt") {
            return Some(existing);
        } else {
            return None;
        }
    }

    None
}

fn install_dir() -> PathBuf {
    Workspace::tools_dir().join(format!("binaryen-{BINARYEN_VERSION}"))
}

fn installed_bin_name() -> &'static str {
    if cfg!(windows) {
        "wasm-opt.exe"
    } else {
        "wasm-opt"
    }
}

fn installed_bin_path(install_dir: &Path) -> PathBuf {
    install_dir.join("bin").join(installed_bin_name())
}

/// Install wasm-opt from GitHub releases into the specified directory
async fn install_github(install_dir: &Path) -> anyhow::Result<()> {
    tracing::trace!("Attempting to install wasm-opt from GitHub");

    std::fs::create_dir_all(install_dir)?;

    let url = find_latest_wasm_opt_download_url()
        .await
        .context("Failed to find latest wasm-opt download URL")?;
    tracing::trace!("Downloading wasm-opt from {}", url);

    // Download the binaryen release archive into memory
    let bytes = crate::http_client::client()
        .get(url)
        .send()
        .await?
        .bytes()
        .await?;

    // We don't need the whole gzip archive, just the wasm-opt binary and the lib folder. We
    // just extract those files from the archive.
    let installed_bin_path = installed_bin_path(install_dir);
    let lib_folder_name = "lib";
    let installed_lib_path = install_dir.join(lib_folder_name);

    // Create the lib and bin directories if they don't exist
    for path in [installed_bin_path.parent(), Some(&installed_lib_path)]
        .into_iter()
        .flatten()
    {
        std::fs::create_dir_all(path)
            .context(format!("Failed to create directory: {}", path.display()))?;
    }

    let mut archive = Archive::new(GzDecoder::new(bytes.as_ref()));

    // Unpack the binary and library files from the archive
    for mut entry in archive.entries()?.flatten() {
        // Unpack the wasm-opt binary
        if entry
            .path_bytes()
            .ends_with(installed_bin_name().as_bytes())
        {
            entry.unpack(&installed_bin_path)?;
        }
        // Unpack any files in the lib folder
        else if let Ok(path) = entry.path() {
            if path.components().any(|c| c.as_os_str() == lib_folder_name) {
                if let Some(file_name) = path.file_name() {
                    entry.unpack(installed_lib_path.join(file_name))?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHILD_OUTPUT: &str = "DIOXUS_TEST_WASM_OPT_OUTPUT";
    const CHILD_MODE: &str = "DIOXUS_TEST_WASM_OPT_MODE";
    const ORIGINAL: &[u8] = b"original wasm";
    const OPTIMIZED: &[u8] = b"optimized wasm";

    // Run only in a child process when exercising optimizer exit handling.
    #[test]
    fn optimizer_process() {
        let Some(output) = std::env::var_os(CHILD_OUTPUT) else {
            return;
        };
        let mode = std::env::var(CHILD_MODE).unwrap();
        std::fs::write(output, OPTIMIZED).unwrap();
        println!("optimizer stdout");
        eprintln!("optimizer stderr");
        match mode.as_str() {
            "success" => {}
            "failure" => std::process::exit(23),
            #[cfg(unix)]
            "signal" => {
                // Terminate by signal without creating a core dump.
                std::process::Command::new("kill")
                    .arg("-TERM")
                    .arg(std::process::id().to_string())
                    .status()
                    .unwrap();
                panic!("the optimizer subprocess should have terminated");
            }
            _ => panic!("unknown optimizer mode"),
        }
    }

    fn optimizer(dir: &Path, in_place: bool) -> WasmOpt {
        let input_path = dir.join("input.wasm");
        std::fs::write(&input_path, ORIGINAL).unwrap();
        WasmOpt {
            path: std::env::current_exe().unwrap(),
            output_path: if in_place {
                input_path.clone()
            } else {
                dir.join("output.wasm")
            },
            input_path,
            temporary_output_path: NamedTempFile::new_in(dir).unwrap(),
            cfg: WasmOptConfig::default(),
        }
    }

    fn command(optimizer: &WasmOpt, mode: &str) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(&optimizer.path);
        command
            .args([
                "--exact",
                "wasm_opt::tests::optimizer_process",
                "--nocapture",
            ])
            .env(CHILD_OUTPUT, optimizer.temporary_output_path.path())
            .env(CHILD_MODE, mode);
        command
    }

    async fn assert_failure(mode: &str, status: &str) {
        for in_place in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let optimizer = optimizer(dir.path(), in_place);
            if !in_place {
                std::fs::write(&optimizer.output_path, b"previous output").unwrap();
            }

            let error = optimizer
                .run(command(&optimizer, mode))
                .await
                .expect_err("a failed optimizer must fail the build")
                .to_string();
            assert!(error.contains("wasm-opt"), "{error}");
            assert!(error.contains(status), "{error}");
            assert!(error.contains("optimizer stderr"), "{error}");
            assert!(error.contains("optimizer stdout"), "{error}");
            assert_eq!(std::fs::read(&optimizer.input_path).unwrap(), ORIGINAL);
            if !in_place {
                assert_eq!(
                    std::fs::read(&optimizer.output_path).unwrap(),
                    b"previous output"
                );
            }
        }
    }

    #[tokio::test]
    async fn unsuccessful_optimizer_fails_build() {
        assert_failure("failure", "23").await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn signaled_optimizer_fails_build() {
        assert_failure("signal", "signal").await;
    }

    #[tokio::test]
    async fn successful_optimizer_copies_output() {
        for in_place in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            let optimizer = optimizer(dir.path(), in_place);
            optimizer.run(command(&optimizer, "success")).await.unwrap();
            assert_eq!(std::fs::read(&optimizer.output_path).unwrap(), OPTIMIZED);
            if !in_place {
                assert_eq!(std::fs::read(&optimizer.input_path).unwrap(), ORIGINAL);
            }
        }
    }

    #[tokio::test]
    async fn output_copy_failure_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let mut optimizer = optimizer(dir.path(), false);
        optimizer.output_path = dir.path().to_path_buf();
        let error = optimizer
            .run(command(&optimizer, "success"))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Failed to write optimized wasm"));
        assert_eq!(std::fs::read(&optimizer.input_path).unwrap(), ORIGINAL);
    }

    #[test]
    fn zero_level_still_passes_o0_to_optimizer() {
        let dir = tempfile::tempdir().unwrap();
        let mut optimizer = optimizer(dir.path(), true);
        optimizer.cfg.level = WasmOptLevel::Zero;
        let command = optimizer.build_command();
        assert!(command.as_std().get_args().any(|arg| arg == "-O0"));
    }
}
