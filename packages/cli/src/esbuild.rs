//! esbuild binary download, caching, and invocation.
//!
//! esbuild is used for JavaScript bundling and minification, replacing the
//! previous SWC Rust library dependencies. The standalone Go binary is
//! downloaded from the npm registry and cached locally.

use crate::{CliSettings, Workspace};
use anyhow::{anyhow, Context};
use std::path::PathBuf;

/// Pinned esbuild version.
const ESBUILD_VERSION: &str = "0.27.3";

pub(crate) struct Esbuild;

impl Esbuild {
    /// Ensure the esbuild binary is available, downloading if needed.
    /// Returns the path to the binary.
    pub(crate) async fn get_or_install() -> anyhow::Result<PathBuf> {
        if CliSettings::prefer_no_downloads() {
            which::which("esbuild")
                .map_err(|_| anyhow!("esbuild not found on PATH and downloads are disabled"))
        } else {
            let path = Self::installed_bin_path();
            if !path.exists() {
                Self::install_from_npm().await?;
            }
            Ok(path)
        }
    }

    /// Return the esbuild binary path if it's already been installed or is on PATH.
    /// This is a sync check intended for use after `verify_tooling` has been called.
    pub(crate) fn path_if_installed() -> Option<PathBuf> {
        let path = Self::installed_bin_path();
        if path.exists() {
            Some(path)
        } else {
            which::which("esbuild").ok()
        }
    }

    /// The path where we cache the esbuild binary.
    fn installed_bin_path() -> PathBuf {
        let dir = Workspace::tools_dir().join(format!("esbuild-{ESBUILD_VERSION}"));
        let name = if cfg!(windows) {
            "esbuild.exe"
        } else {
            "esbuild"
        };
        dir.join(name)
    }

    /// Download esbuild from the npm registry and extract the binary.
    ///
    /// esbuild publishes platform-specific packages to npm as tar.gz archives.
    /// Each archive contains `package/bin/esbuild[.exe]`.
    async fn install_from_npm() -> anyhow::Result<()> {
        let platform = Self::npm_platform_package()
            .ok_or_else(|| anyhow!("No esbuild binary available for this platform"))?;

        let url = format!(
            "https://registry.npmjs.org/@esbuild/{platform}/-/{platform}-{ESBUILD_VERSION}.tgz"
        );

        tracing::info!("Installing esbuild v{ESBUILD_VERSION} from {url}...");

        let bytes = reqwest::get(&url)
            .await
            .with_context(|| format!("Failed to download esbuild from {url}"))?
            .bytes()
            .await
            .context("Failed to read esbuild download response")?;

        // Extract `package/bin/esbuild[.exe]` from the tar.gz
        let binary_data = Self::extract_binary_from_tgz(&bytes)?;

        let binary_path = Self::installed_bin_path();
        std::fs::create_dir_all(binary_path.parent().unwrap())
            .context("Failed to create esbuild cache directory")?;

        std::fs::write(&binary_path, &binary_data).context("Failed to write esbuild binary")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = binary_path.metadata()?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)?;
        }

        tracing::info!("esbuild v{ESBUILD_VERSION} installed successfully");
        Ok(())
    }

    /// Extract the esbuild binary from an npm tar.gz package.
    ///
    /// The binary is at `package/bin/esbuild` (or `package/bin/esbuild.exe` on Windows).
    fn extract_binary_from_tgz(tgz_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let bin_name = if cfg!(windows) {
            "esbuild.exe"
        } else {
            "esbuild"
        };
        Self::extract_binary_named_from_tgz(tgz_bytes, bin_name)
    }

    fn extract_binary_named_from_tgz(tgz_bytes: &[u8], bin_name: &str) -> anyhow::Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        use tar::Archive;

        let decoder = GzDecoder::new(tgz_bytes);
        let mut archive = Archive::new(decoder);

        let expected_paths = [
            format!("package/bin/{bin_name}"),
            format!("package/{bin_name}"),
        ];
        let mut archive_entries = Vec::new();

        for entry in archive.entries().context("Failed to read tar entries")? {
            let mut entry = entry.context("Failed to read tar entry")?;
            let path = entry.path().context("Failed to read entry path")?;
            let path_string = path.to_string_lossy().replace('\\', "/");

            if expected_paths.contains(&path_string) {
                let mut data = Vec::new();
                entry
                    .read_to_end(&mut data)
                    .context("Failed to read esbuild binary from archive")?;
                return Ok(data);
            }

            archive_entries.push(path_string);
        }

        archive_entries.sort();
        archive_entries.truncate(10);

        anyhow::bail!(
            "esbuild binary not found in archive (expected one of {}). Found entries: {}",
            expected_paths.join(", "),
            archive_entries.join(", ")
        );
    }

    /// Map the host platform to the npm package name for esbuild.
    ///
    /// esbuild publishes per-platform packages under `@esbuild/{name}`:
    /// - darwin-arm64, darwin-x64
    /// - linux-x64, linux-arm64
    /// - win32-x64, win32-arm64
    fn npm_platform_package() -> Option<&'static str> {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Some("darwin-arm64")
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            Some("darwin-x64")
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Some("linux-x64")
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            Some("linux-arm64")
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Some("win32-x64")
        } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
            Some("win32-arm64")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Esbuild;
    use flate2::{write::GzEncoder, Compression};

    fn tgz_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut tar = tar::Builder::new(&mut encoder);
            for (path, contents) in entries {
                let mut header = tar::Header::new_gnu();
                header.set_size(contents.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                tar.append_data(&mut header, *path, *contents).unwrap();
            }
            tar.finish().unwrap();
        }
        encoder.finish().unwrap()
    }

    #[test]
    fn extracts_binary_from_bin_layout() {
        let tgz = tgz_with_entries(&[("package/bin/esbuild.exe", b"windows-binary")]);
        let extracted = Esbuild::extract_binary_named_from_tgz(&tgz, "esbuild.exe").unwrap();
        assert_eq!(extracted, b"windows-binary");
    }

    #[test]
    fn extracts_binary_from_package_root_layout() {
        let tgz = tgz_with_entries(&[("package/esbuild.exe", b"windows-binary")]);
        let extracted = Esbuild::extract_binary_named_from_tgz(&tgz, "esbuild.exe").unwrap();
        assert_eq!(extracted, b"windows-binary");
    }
}
