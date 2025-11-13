//! The cargo_toml crate contains some logic for resolving Cargo.toml files with workspace inheritance, but it
//! doesn't handle global configs like ~/.cargo/config.toml. This module handles extending the manifest with those
//! settings if they exist.

use std::path::{Path, PathBuf};

use cargo_toml::{Manifest, Profile, Profiles};

/// Load the manifest from a path inheriting from the global config where needed
pub fn load_manifest_from_path(path: &Path) -> Result<Manifest, cargo_toml::Error> {
    let mut original = Manifest::from_path(path)?;

    // Merge the .cargo/config.toml if it exists
    extend_manifest_config_toml(&mut original, &path.join(".cargo").join("config.toml"));

    // Merge the global cargo config if it exists
    if let Some(global_config) = global_cargo_config_path() {
        extend_manifest_config_toml(&mut original, &global_config);
    }

    Ok(original)
}

/// Get the path to cargo home
fn cargo_home() -> Option<PathBuf> {
    // If the cargo home env var is set, use that
    if let Some(cargo_home) = std::env::var_os("CARGO_HOME") {
        return Some(PathBuf::from(cargo_home));
    }

    // Otherwise, use the default location
    if cfg!(windows) {
        std::env::var_os("USERPROFILE")
            .map(|user_profile| PathBuf::from(user_profile).join(".cargo"))
    } else if cfg!(unix) {
        dirs::home_dir().map(|home_dir| home_dir.join(".cargo"))
    } else {
        None
    }
}

/// Get the global cargo config path if it exists
fn global_cargo_config_path() -> Option<PathBuf> {
    cargo_home().map(|cargo_home| cargo_home.join("config.toml"))
}

// Extend a manifest with a config.toml if it exists
fn extend_manifest_config_toml(manifest: &mut Manifest, path: &Path) {
    // Read the config.toml if it exists
    let Ok(config) = std::fs::read_to_string(path) else {
        return;
    };

    let Ok(config) = config.parse::<toml::Value>() else {
        return;
    };

    // Try to parse profiles
    if let Some(profiles) = config.get("profile").and_then(|p| p.as_table()) {
        merge_profiles(
            &mut manifest.profile,
            toml::from_str::<cargo_toml::Profiles>(&profiles.to_string()).unwrap_or_default(),
        );
    }
}

/// Merge the new profiles into the target profiles. Keep the existing values if they exist.
fn merge_profiles(target: &mut Profiles, new: Profiles) {
    if let Some(new_release) = new.release {
        if target.release.is_none() {
            target.release = Some(new_release);
        } else {
            merge_profile(target.release.as_mut().unwrap(), new_release);
        }
    }

    if let Some(new_dev) = new.dev {
        if target.dev.is_none() {
            target.dev = Some(new_dev);
        } else {
            merge_profile(target.dev.as_mut().unwrap(), new_dev);
        }
    }

    if let Some(new_test) = new.test {
        if target.test.is_none() {
            target.test = Some(new_test);
        } else {
            merge_profile(target.test.as_mut().unwrap(), new_test);
        }
    }

    if let Some(new_bench) = new.bench {
        if target.bench.is_none() {
            target.bench = Some(new_bench);
        } else {
            merge_profile(target.bench.as_mut().unwrap(), new_bench);
        }
    }

    #[allow(deprecated)]
    if let Some(new_doc) = new.doc {
        if target.doc.is_none() {
            target.doc = Some(new_doc);
        } else {
            merge_profile(target.doc.as_mut().unwrap(), new_doc);
        }
    }

    for (profile_name, profile) in new.custom {
        if let Some(target_profile) = target.custom.get_mut(&profile_name) {
            merge_profile(target_profile, profile);
        } else {
            target.custom.insert(profile_name, profile);
        }
    }
}

/// Merge the new profile into the target profile. Keep the existing values if they exist.
fn merge_profile(target: &mut Profile, new: Profile) {
    if target.opt_level.is_none() {
        target.opt_level = new.opt_level;
    }
    if target.debug.is_none() {
        target.debug = new.debug;
    }
    if target.split_debuginfo.is_none() {
        target.split_debuginfo = new.split_debuginfo;
    }
    if target.rpath.is_none() {
        target.rpath = new.rpath;
    }
    if target.lto.is_none() {
        target.lto = new.lto;
    }
    if target.debug_assertions.is_none() {
        target.debug_assertions = new.debug_assertions;
    }
    if target.codegen_units.is_none() {
        target.codegen_units = new.codegen_units;
    }
    if target.panic.is_none() {
        target.panic = new.panic;
    }
    if target.incremental.is_none() {
        target.incremental = new.incremental;
    }
    if target.overflow_checks.is_none() {
        target.overflow_checks = new.overflow_checks;
    }
    if target.strip.is_none() {
        target.strip = new.strip;
    }
    if target.build_override.is_none() {
        target.build_override = new.build_override;
    }
    if target.inherits.is_none() {
        target.inherits = new.inherits;
    }
    target.package.extend(new.package);
}
