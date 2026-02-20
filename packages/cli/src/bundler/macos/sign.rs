use crate::MacOsSettings;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A code signing identity, optionally backed by a temporary keychain.
pub(crate) struct SigningIdentity {
    /// The identity string passed to `codesign --sign`.
    /// This is either a team/certificate name or a SHA-1 hash.
    pub identity: String,

    /// If we created a temporary keychain to import a certificate,
    /// this holds its path so we can clean it up later.
    pub temp_keychain: Option<TempKeychain>,
}

/// A temporary keychain created for CI certificate import.
#[allow(dead_code)]
pub(crate) struct TempKeychain {
    pub path: PathBuf,
    pub password: String,
}

impl Drop for TempKeychain {
    fn drop(&mut self) {
        tracing::debug!("Cleaning up temporary keychain: {}", self.path.display());
        let _ = Command::new("security")
            .args(["delete-keychain", &self.path.display().to_string()])
            .status();
    }
}

/// A target to be code-signed.
pub(crate) struct SignTarget {
    pub path: PathBuf,
    pub is_an_executable: bool,
}

/// Set up the signing identity.
///
/// This checks for the `APPLE_CERTIFICATE` and `APPLE_CERTIFICATE_PASSWORD`
/// environment variables first (for CI). If those are present, the certificate
/// is imported into a temporary keychain.
///
/// If those env vars are not set, falls back to the provided `identity` string
/// (typically from `MacOsSettings::signing_identity`).
///
/// Returns `None` if no signing identity is available.
pub(crate) fn setup_keychain(identity: Option<&str>) -> Result<Option<SigningIdentity>> {
    let certificate_encoded = std::env::var("APPLE_CERTIFICATE").ok();
    let certificate_password = std::env::var("APPLE_CERTIFICATE_PASSWORD")
        .ok()
        .unwrap_or_default();

    if let Some(cert_base64) = certificate_encoded {
        tracing::info!("Setting up temporary keychain for code signing (CI mode)");
        let keychain = setup_temp_keychain(&cert_base64, &certificate_password)?;
        // The signing identity to use when we imported a certificate.
        // We need to find the identity from the imported cert.
        // Use `security find-identity` to extract it.
        let identity_name = find_identity_in_keychain(&keychain.path)?;
        return Ok(Some(SigningIdentity {
            identity: identity_name,
            temp_keychain: Some(keychain),
        }));
    }

    if let Some(id) = identity {
        if !id.is_empty() {
            return Ok(Some(SigningIdentity {
                identity: id.to_string(),
                temp_keychain: None,
            }));
        }
    }

    Ok(None)
}

/// Set up a temporary keychain and import the certificate into it.
fn setup_temp_keychain(cert_base64: &str, password: &str) -> Result<TempKeychain> {
    use std::io::Write;

    let keychain_password = "dioxus-bundle-keychain";
    let keychain_path = std::env::temp_dir().join("dioxus-signing.keychain-db");

    // Decode the certificate
    let cert_data = base64_decode(cert_base64)
        .context("Failed to decode APPLE_CERTIFICATE from base64")?;

    // Write the certificate to a temp file
    let cert_file = std::env::temp_dir().join("dioxus-signing-cert.p12");
    let mut f = std::fs::File::create(&cert_file)?;
    f.write_all(&cert_data)?;
    drop(f);

    // Delete any old keychain with the same name
    let _ = Command::new("security")
        .args(["delete-keychain", &keychain_path.display().to_string()])
        .output();

    // Create the keychain
    run_command(
        Command::new("security").args([
            "create-keychain",
            "-p",
            keychain_password,
            &keychain_path.display().to_string(),
        ]),
        "create-keychain",
    )?;

    // Import the certificate
    run_command(
        Command::new("security").args([
            "import",
            &cert_file.display().to_string(),
            "-k",
            &keychain_path.display().to_string(),
            "-P",
            password,
            "-T",
            "/usr/bin/codesign",
            "-T",
            "/usr/bin/security",
        ]),
        "import certificate",
    )?;

    // Add the keychain to the search list
    // First, get the current list to preserve it
    let output = Command::new("security")
        .args(["list-keychains", "-d", "user"])
        .output()
        .context("Failed to list keychains")?;
    let current_keychains = String::from_utf8_lossy(&output.stdout);
    let mut keychains: Vec<String> = current_keychains
        .lines()
        .map(|l| l.trim().trim_matches('"').to_string())
        .filter(|l| !l.is_empty())
        .collect();
    keychains.insert(0, keychain_path.display().to_string());

    let mut cmd = Command::new("security");
    cmd.args(["list-keychains", "-d", "user", "-s"]);
    for kc in &keychains {
        cmd.arg(kc);
    }
    run_command(&mut cmd, "list-keychains -s")?;

    // Unlock the keychain
    run_command(
        Command::new("security").args([
            "unlock-keychain",
            "-p",
            keychain_password,
            &keychain_path.display().to_string(),
        ]),
        "unlock-keychain",
    )?;

    // Set the key partition list to allow codesign access without UI prompt
    run_command(
        Command::new("security").args([
            "set-key-partition-list",
            "-S",
            "apple-tool:,apple:",
            "-s",
            "-k",
            keychain_password,
            &keychain_path.display().to_string(),
        ]),
        "set-key-partition-list",
    )?;

    // Clean up temp cert file
    let _ = std::fs::remove_file(&cert_file);

    Ok(TempKeychain {
        path: keychain_path,
        password: keychain_password.to_string(),
    })
}

/// Find the signing identity in a keychain using `security find-identity`.
fn find_identity_in_keychain(keychain_path: &Path) -> Result<String> {
    let output = Command::new("security")
        .args([
            "find-identity",
            "-v",
            "-p",
            "codesigning",
            &keychain_path.display().to_string(),
        ])
        .output()
        .context("Failed to run `security find-identity`")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the output: lines look like:
    //   1) ABCDEF1234... "Developer ID Application: Name (TEAMID)"
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("1)") || line.contains("Developer ID") || line.contains("Apple Development") {
            // Extract the quoted identity string
            if let Some(start) = line.find('"') {
                if let Some(end) = line.rfind('"') {
                    if end > start {
                        return Ok(line[start + 1..end].to_string());
                    }
                }
            }
            // If no quotes, try extracting the hash (40-char hex after the number)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].len() == 40 {
                return Ok(parts[1].to_string());
            }
        }
    }

    bail!(
        "No valid signing identity found in keychain {}.\nOutput: {}",
        keychain_path.display(),
        stdout
    )
}

/// Sign a list of paths with the given identity.
pub(crate) fn sign_paths(
    identity: &SigningIdentity,
    targets: Vec<SignTarget>,
    settings: &MacOsSettings,
) -> Result<()> {
    for target in &targets {
        sign_path(identity, target, settings)?;
    }
    Ok(())
}

/// Sign a single path with `codesign`.
fn sign_path(
    identity: &SigningIdentity,
    target: &SignTarget,
    settings: &MacOsSettings,
) -> Result<()> {
    tracing::info!("Signing: {}", target.path.display());

    let mut cmd = Command::new("codesign");
    cmd.args(["--force", "--sign", &identity.identity]);

    // --options runtime enables the hardened runtime
    if settings.hardened_runtime {
        cmd.arg("--options");
        cmd.arg("runtime");
    }

    // --deep for .app bundles and frameworks
    if target.path.extension().map(|e| e == "app" || e == "framework").unwrap_or(false) {
        cmd.arg("--deep");
    }

    // Entitlements
    if let Some(entitlements) = &settings.entitlements {
        cmd.args(["--entitlements", entitlements]);
    }

    // If we have a temp keychain, specify it
    if let Some(keychain) = &identity.temp_keychain {
        cmd.args(["--keychain", &keychain.path.display().to_string()]);
    }

    cmd.arg(&target.path);

    run_command(&mut cmd, &format!("codesign {}", target.path.display()))?;
    Ok(())
}

/// Notarize a .app or .dmg with Apple's notary service.
///
/// Requires the following environment variables:
/// - `APPLE_ID`: Apple ID email
/// - `APPLE_PASSWORD`: App-specific password or `@keychain:label`
/// - `APPLE_TEAM_ID`: Team ID
///
/// Or for API key-based notarization:
/// - `APPLE_API_KEY`: API key ID
/// - `APPLE_API_ISSUER`: API key issuer ID
/// - `APPLE_API_KEY_PATH`: Path to the .p8 key file
pub(crate) fn notarize(app_path: &Path) -> Result<()> {
    let apple_id = std::env::var("APPLE_ID").ok();
    let apple_password = std::env::var("APPLE_PASSWORD").ok();
    let apple_team_id = std::env::var("APPLE_TEAM_ID").ok();

    let api_key = std::env::var("APPLE_API_KEY").ok();
    let api_issuer = std::env::var("APPLE_API_ISSUER").ok();
    let api_key_path = std::env::var("APPLE_API_KEY_PATH").ok();

    let mut cmd = Command::new("xcrun");
    cmd.args(["notarytool", "submit"]);
    cmd.arg(app_path);

    if let (Some(key), Some(issuer), Some(key_path)) = (&api_key, &api_issuer, &api_key_path) {
        // API key-based notarization
        cmd.args(["--key", key_path]);
        cmd.args(["--key-id", key]);
        cmd.args(["--issuer", issuer]);
    } else if let (Some(id), Some(pwd), Some(team)) = (&apple_id, &apple_password, &apple_team_id)
    {
        // Apple ID-based notarization
        cmd.args(["--apple-id", id]);
        cmd.args(["--password", pwd]);
        cmd.args(["--team-id", team]);
    } else {
        bail!(
            "Notarization requires either:\n\
             - APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID env vars, or\n\
             - APPLE_API_KEY, APPLE_API_ISSUER, and APPLE_API_KEY_PATH env vars"
        );
    }

    cmd.arg("--wait");

    tracing::info!("Submitting {} for notarization...", app_path.display());
    run_command(&mut cmd, "xcrun notarytool submit")?;

    // Staple the notarization ticket to the app
    tracing::info!("Stapling notarization ticket...");
    run_command(
        Command::new("xcrun").args(["stapler", "staple"]).arg(app_path),
        "xcrun stapler staple",
    )?;

    tracing::info!("Notarization complete for {}", app_path.display());
    Ok(())
}

/// Helper to run a command and return a nice error on failure.
fn run_command(cmd: &mut Command, description: &str) -> Result<()> {
    tracing::debug!("Running: {:?}", cmd);
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute `{description}`"))?;

    if !status.success() {
        bail!("`{description}` failed with exit code: {}", status);
    }
    Ok(())
}

/// Decode base64 (standard or URL-safe).
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    // Simple base64 decode without pulling in a base64 crate.
    // We shell out to `base64` which is available on macOS.
    let output = Command::new("base64")
        .args(["--decode"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input.as_bytes())?;
            }
            child.wait_with_output()
        })
        .context("Failed to decode base64 certificate")?;

    if !output.status.success() {
        bail!(
            "base64 --decode failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(output.stdout)
}
