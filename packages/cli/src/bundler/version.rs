use anyhow::{Context, Result, bail};
use krates::semver::Version;

fn core(version: &Version) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}

/// Shared RPM/Debian projection: `~` for prereleases, hyphens folded, build metadata dropped.
fn linux(version: &Version) -> String {
    let mut projected = core(version);
    if !version.pre.is_empty() {
        projected.push('~');
        projected.push_str(&version.pre.as_str().replace('-', "."));
    }
    projected
}

/// Convert a SemVer version into an RPM-compatible version.
pub(crate) fn rpm(version: &Version) -> String {
    linux(version)
}

/// Convert a SemVer version into a Debian-compatible version.
pub(crate) fn deb(version: &Version) -> String {
    linux(version)
}

/// Convert a SemVer version into a WiX-compatible product version.
pub(crate) fn wix(version: &Version) -> Result<String> {
    wix_validate_component(version.major, 0)?;
    wix_validate_component(version.minor, 1)?;
    wix_validate_component(version.patch, 2)?;
    Ok(core(version))
}

/// Validate and normalize an explicitly configured WiX product version.
pub(crate) fn wix_from_string(version: &str) -> Result<String> {
    let version = version.split('-').next().unwrap_or(version);
    let parts: Vec<&str> = version.split('.').collect();

    if parts.len() < 2 || parts.len() > 4 {
        bail!(
            "Invalid version for MSI: '{}'. Expected format: major.minor.patch[.build]",
            version
        );
    }

    for (index, part) in parts.iter().enumerate() {
        let value: u64 = part
            .parse()
            .with_context(|| format!("Invalid version component: '{part}'"))?;
        wix_validate_component(value, index)?;
    }

    if parts.len() == 2 {
        Ok(format!("{}.{}.0", parts[0], parts[1]))
    } else {
        Ok(parts.join("."))
    }
}

fn wix_validate_component(value: u64, index: usize) -> Result<()> {
    match index {
        0 | 1 if value > 255 => {
            bail!("Version component {value} exceeds maximum value of 255")
        }
        2 | 3 if value > 65535 => {
            bail!("Version component {value} exceeds maximum value of 65535")
        }
        _ => Ok(()),
    }
}

/// Convert a SemVer version into an NSIS-compatible product version.
///
/// The NSIS template appends `.0` to form `VIProductVersion`.
pub(crate) fn nsis(version: &Version) -> String {
    core(version)
}

/// Convert a SemVer version into a macOS bundle version (`CFBundleShortVersionString`).
pub(crate) fn macos(version: &Version) -> String {
    core(version)
}

#[cfg(test)]
mod tests {
    use super::{deb, macos, nsis, rpm, wix, wix_from_string};
    use krates::semver::Version;

    fn version(value: &str) -> Version {
        Version::parse(value).unwrap()
    }

    #[test]
    fn linux_formats_share_a_projection() {
        assert_eq!(rpm(&version("1.2.3")), "1.2.3");
        assert_eq!(rpm(&version("1.2.3-rc.1")), "1.2.3~rc.1");
        assert_eq!(rpm(&version("1.2.3-alpha-1+build-5")), "1.2.3~alpha.1");
        assert_eq!(rpm(&version("1.2.3+build.5")), "1.2.3");
        assert_eq!(deb(&version("1.2.3-rc.1")), rpm(&version("1.2.3-rc.1")));
        assert_eq!(
            deb(&version("1.2.3-alpha-1+build-5")),
            rpm(&version("1.2.3-alpha-1+build-5"))
        );
    }

    #[test]
    fn macos_and_nsis_use_the_numeric_core() {
        assert_eq!(macos(&version("1.2.3-rc.1+build.5")), "1.2.3");
        assert_eq!(nsis(&version("1.2.3-rc.1+build.5")), "1.2.3");
        assert_eq!(nsis(&version("256.2.3")), "256.2.3");
    }

    #[test]
    fn wix_projects_semver_versions() {
        assert_eq!(wix(&version("1.2.3")).unwrap(), "1.2.3");
        assert_eq!(wix(&version("1.2.3-rc.1+build.5")).unwrap(), "1.2.3");
        assert!(wix(&version("256.2.3")).is_err());
        assert!(wix(&version("1.256.3")).is_err());
        assert!(wix(&version("1.2.65536")).is_err());
    }

    #[test]
    fn wix_from_string_preserves_explicit_version_compatibility() {
        assert_eq!(wix_from_string("1.2").unwrap(), "1.2.0");
        assert_eq!(wix_from_string("1.2.3.4").unwrap(), "1.2.3.4");
        assert_eq!(wix_from_string("1.2.3-rc.1").unwrap(), "1.2.3");
        assert!(wix_from_string("1.2.3+build.5").is_err());
        assert!(wix_from_string("1").is_err());
        assert!(wix_from_string("1.2.3.4.5").is_err());
        assert!(wix_from_string("1.2.invalid").is_err());
    }
}
