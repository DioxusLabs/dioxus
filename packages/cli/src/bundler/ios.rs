use super::{copy_dir_recursive, zip_dir_recursive, Arch, Bundle, BundleContext};
use crate::PackageType;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub(crate) struct IosBundled {
    pub ipa: Vec<PathBuf>,
    #[allow(dead_code)]
    pub app: Vec<PathBuf>,
}

impl BundleContext<'_> {
    /// Return the already-assembled iOS `.app` bundle produced by the build pipeline.
    pub(crate) async fn bundle_ios_app(&self) -> Result<Vec<PathBuf>> {
        let app_path = self.build.root_dir();
        if !app_path.exists() {
            bail!(
                "iOS app bundle not found at {}. Ensure the iOS build completed successfully.",
                app_path.display()
            );
        }

        Ok(vec![app_path])
    }

    /// Package a signed iOS `.app` into an App Store distributable `.ipa`.
    pub(crate) async fn bundle_ios_ipa(&self, bundles: &[Bundle]) -> Result<IosBundled> {
        validate_ios_ipa_target(&self.target(), self.binary_arch())?;

        let (app_paths, synthesized_app_paths) = if let Some(app_bundle) = bundles
            .iter()
            .find(|bundle| bundle.package_type == PackageType::IosApp)
        {
            (app_bundle.bundle_paths.clone(), Vec::new())
        } else {
            let paths = self.bundle_ios_app().await?;
            (paths.clone(), paths)
        };

        if app_paths.is_empty() {
            bail!("No iOS .app bundle found to package into an .ipa");
        }

        let app_path = &app_paths[0];
        if !app_path.exists() {
            bail!(
                "iOS .app bundle does not exist at expected path: {}",
                app_path.display()
            );
        }

        verify_codesigned_app(app_path).await?;

        let output_dir = self.project_out_directory().join("bundle").join("ipa");
        std::fs::create_dir_all(&output_dir)?;

        let ipa_name = format!(
            "{}_{}_{}.ipa",
            self.product_name(),
            self.version_string(),
            self.binary_arch()
        );
        let output_path = output_dir.join(ipa_name);

        tracing::info!("Creating iOS IPA at {}", output_path.display());

        let staging_dir =
            tempfile::tempdir().context("Failed to create temp dir for IPA staging")?;
        let payload_dir = staging_dir.path().join("Payload");
        std::fs::create_dir_all(&payload_dir)?;

        let staged_app = payload_dir.join(
            app_path
                .file_name()
                .context("iOS .app bundle is missing a file name")?,
        );
        copy_dir_recursive(app_path, &staged_app)?;

        if output_path.exists() {
            std::fs::remove_file(&output_path)?;
        }
        zip_dir_recursive(staging_dir.path(), &output_path)?;

        tracing::info!("IPA created at {}", output_path.display());

        Ok(IosBundled {
            ipa: vec![output_path],
            app: synthesized_app_paths,
        })
    }
}

fn validate_ios_ipa_target(target: &str, arch: Arch) -> Result<()> {
    let is_device = arch == Arch::AArch64 && !target.contains("sim");
    if !is_device {
        bail!(
            "IPA packaging requires a physical-device iOS target. Resolved target was `{target}`."
        );
    }

    Ok(())
}

async fn verify_codesigned_app(app_path: &Path) -> Result<()> {
    verify_codesigned_app_with(Path::new("codesign"), app_path).await
}

async fn verify_codesigned_app_with(codesign: &Path, app_path: &Path) -> Result<()> {
    let output = Command::new(codesign)
        .args(["--verify", "--deep", "--strict"])
        .arg(app_path)
        .output()
        .await
        .with_context(|| format!("Failed to run `{}`", codesign.display()))?;

    if !output.status.success() {
        bail!(
            "iOS .app bundle must be codesigned before creating an .ipa: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_ios_ipa_target, verify_codesigned_app_with};
    use crate::bundler::Arch;

    #[test]
    fn ipa_rejects_simulator_targets() {
        assert!(validate_ios_ipa_target("x86_64-apple-ios", Arch::X86).is_err());
        assert!(validate_ios_ipa_target("aarch64-apple-ios-sim", Arch::AArch64).is_err());
        assert!(validate_ios_ipa_target("aarch64-apple-ios", Arch::AArch64).is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn verify_codesign_helper_reports_failure() {
        let temp = tempfile::tempdir().unwrap();
        let app_path = temp.path().join("App.app");
        std::fs::create_dir_all(&app_path).unwrap();

        let script = temp.path().join("fake-codesign.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf 'unsigned bundle' >&2\nexit 1\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let err = verify_codesigned_app_with(&script, &app_path)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("codesigned"));
    }
}
