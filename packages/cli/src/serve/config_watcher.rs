//! Detects edits to `Cargo.toml` and `Dioxus.toml` while `dx serve` is running and decides
//! whether to trigger a full rebuild, warn the user that a restart is required, or ignore the
//! change as cosmetic / irrelevant to the active build.
//!
//! The classification is field-aware: we parse the file as `toml::Value` and compare a curated
//! set of subtrees. Whitespace, comments, key reordering, and edits to fields outside the curated
//! set show up as `Ignore`, so a stray edit to e.g. `package.description` doesn't kick off a
//! 30-second rebuild.
//!
//! Profile and platform sections are filtered against the *active* profile and bundle. Editing
//! `[profile.release]` while serving in `dev` produces an `Ignore` with a debug note rather than
//! a rebuild — likewise for `[ios]` settings while serving for the web.
//!
//! See [`ConfigWatcher::analyze_cargo_toml`] and [`ConfigWatcher::analyze_dioxus_toml`].
use crate::BundleFormat;
use std::{collections::HashMap, path::PathBuf};

/// What the runner should do in response to a config-file edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfigChangeOutcome {
    /// Nothing rebuild-relevant changed (whitespace, comment-only edit, irrelevant field, edit
    /// to an inactive profile/platform). `note` is logged at debug level so silent edits aren't
    /// completely invisible.
    Ignore { note: Option<String> },

    /// A field that affects compilation changed. The runner should kick off a full rebuild and
    /// surface `reason` to the user as the "Full rebuild:" log line.
    FullRebuild { reason: String },

    /// A field that's only consumed at devserver-startup changed (proxy, https, watch paths).
    /// The runner should warn the user that a `dx serve` restart is required to pick up the
    /// change. No build action is taken.
    WarnRestart { reason: String },
}

impl ConfigChangeOutcome {
    /// Combine two outcomes by keeping the strongest action.
    /// `FullRebuild` > `WarnRestart` > `Ignore`.
    pub(crate) fn escalate(self, other: ConfigChangeOutcome) -> ConfigChangeOutcome {
        use ConfigChangeOutcome::*;
        match (&self, &other) {
            (FullRebuild { .. }, _) => self,
            (_, FullRebuild { .. }) => other,
            (WarnRestart { .. }, _) => self,
            (_, WarnRestart { .. }) => other,
            (Ignore { note: Some(_) }, Ignore { note: None }) => self,
            (Ignore { note: None }, Ignore { note: Some(_) }) => other,
            _ => self,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AnalysisCtx {
    pub active_profile: String,
    pub active_bundle: BundleFormat,
}

pub(crate) struct ConfigWatcher {
    cargo_snapshots: HashMap<PathBuf, toml::Value>,
    dioxus_snapshots: HashMap<PathBuf, toml::Value>,
    ctx: AnalysisCtx,
}

impl ConfigWatcher {
    pub(crate) fn new(active_profile: String, active_bundle: BundleFormat) -> Self {
        Self {
            cargo_snapshots: HashMap::new(),
            dioxus_snapshots: HashMap::new(),
            ctx: AnalysisCtx {
                active_profile,
                active_bundle,
            },
        }
    }

    /// Seed a snapshot from the on-disk file. Silently skips files that don't exist or fail to
    /// parse — they'll be picked up the next time they're edited successfully.
    pub(crate) fn seed_cargo(&mut self, path: &std::path::Path) {
        if let Ok(value) = read_toml(path) {
            self.cargo_snapshots.insert(path.to_path_buf(), value);
        }
    }

    pub(crate) fn seed_dioxus(&mut self, path: &std::path::Path) {
        if let Ok(value) = read_toml(path) {
            self.dioxus_snapshots.insert(path.to_path_buf(), value);
        }
    }

    /// Classify a change to `path`, which the caller has already determined is a `Cargo.toml`.
    /// Updates the cached snapshot if the new file parses cleanly.
    pub(crate) fn analyze_cargo_toml(&mut self, path: &std::path::Path) -> ConfigChangeOutcome {
        let new_value = match read_toml(path) {
            Ok(v) => v,
            Err(_) => {
                // Mid-edit (parse error) — keep the old snapshot so the next successful save
                // diffs against the last known-good state.
                return ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Cargo.toml parse failed at {}, will retry on next save",
                        path.display()
                    )),
                };
            }
        };

        let old_value = self
            .cargo_snapshots
            .get(path)
            .cloned()
            .unwrap_or(toml::Value::Table(Default::default()));

        let outcome = analyze_cargo_value(&old_value, &new_value, &self.ctx);
        self.cargo_snapshots.insert(path.to_path_buf(), new_value);
        outcome
    }

    pub(crate) fn analyze_dioxus_toml(&mut self, path: &std::path::Path) -> ConfigChangeOutcome {
        let new_value = match read_toml(path) {
            Ok(v) => v,
            Err(_) => {
                return ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Dioxus.toml parse failed at {}, will retry on next save",
                        path.display()
                    )),
                };
            }
        };

        let old_value = self
            .dioxus_snapshots
            .get(path)
            .cloned()
            .unwrap_or(toml::Value::Table(Default::default()));

        let outcome = analyze_dioxus_value(&old_value, &new_value, &self.ctx);
        self.dioxus_snapshots.insert(path.to_path_buf(), new_value);
        outcome
    }
}

fn read_toml(path: &std::path::Path) -> Result<toml::Value, anyhow::Error> {
    let s = std::fs::read_to_string(path)?;
    let v = toml::from_str::<toml::Value>(&s)?;
    Ok(v)
}

// ---------------------------------------------------------------------------------------------
// Cargo.toml analysis
// ---------------------------------------------------------------------------------------------

fn analyze_cargo_value(
    old: &toml::Value,
    new: &toml::Value,
    ctx: &AnalysisCtx,
) -> ConfigChangeOutcome {
    if old == new {
        return ConfigChangeOutcome::Ignore { note: None };
    }

    let mut outcome = ConfigChangeOutcome::Ignore { note: None };

    // -------- Sections that always force a full rebuild when their contents differ --------
    let rebuild_sections: &[&str] = &[
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "features",
        "lib",
        "bin",
        "example",
        "test",
        "bench",
        "patch",
        "replace",
    ];
    for section in rebuild_sections {
        if get(old, &[section]) != get(new, &[section]) {
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                reason: format!("Cargo.toml [{section}] changed"),
            });
        }
    }

    // -------- target.<cfg>.dependencies / dev-dependencies / build-dependencies --------
    if get(old, &["target"]) != get(new, &["target"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            reason: "Cargo.toml [target.*] dependencies changed".to_string(),
        });
    }

    // -------- [package] subset that affects compilation --------
    let pkg_compile_keys: &[&str] = &[
        "name",
        "version",
        "edition",
        "rust-version",
        "build",
        "default-run",
        "links",
        "autobins",
        "autoexamples",
        "autotests",
        "autobenches",
        "resolver",
    ];
    for key in pkg_compile_keys {
        if get(old, &["package", key]) != get(new, &["package", key]) {
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                reason: format!("Cargo.toml [package].{key} changed"),
            });
        }
    }

    // -------- [profile.<name>] — only relevant if <name> is in the active profile's chain --------
    if let (Some(old_profiles), Some(new_profiles)) = (
        get(old, &["profile"]).and_then(as_table),
        get(new, &["profile"]).and_then(as_table),
    ) {
        let mut all_names: std::collections::BTreeSet<&str> = Default::default();
        all_names.extend(old_profiles.keys().map(String::as_str));
        all_names.extend(new_profiles.keys().map(String::as_str));

        for name in all_names {
            if old_profiles.get(name) == new_profiles.get(name) {
                continue;
            }
            // Use the *new* profile table for inheritance so a freshly-added `inherits` is
            // honored, falling back to the old table for profiles that were just deleted.
            if profile_in_active_chain(name, &ctx.active_profile, new_profiles, old_profiles) {
                outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                    reason: format!("Cargo.toml [profile.{name}] changed"),
                });
            } else {
                outcome = outcome.escalate(ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Saw change to [profile.{name}] but active profile is `{}` — ignoring.",
                        ctx.active_profile
                    )),
                });
            }
        }
    } else if get(old, &["profile"]) != get(new, &["profile"]) {
        // One side missing the [profile] table entirely — treat as full rebuild for safety.
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            reason: "Cargo.toml [profile] section added or removed".to_string(),
        });
    }

    // -------- [workspace] subset that affects build composition --------
    let workspace_compile_keys: &[&str] = &[
        "members",
        "default-members",
        "exclude",
        "resolver",
        "dependencies",
        "package",
        "metadata",
    ];
    for key in workspace_compile_keys {
        if get(old, &["workspace", key]) != get(new, &["workspace", key]) {
            let extra = if *key == "members" || *key == "default-members" {
                " (note: source files in newly-added workspace members won't be hot-reloaded until you restart `dx serve`)"
            } else {
                ""
            };
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                reason: format!("Cargo.toml [workspace].{key} changed{extra}"),
            });
        }
    }

    // -------- workspace-level patch / replace --------
    if get(old, &["workspace", "patch"]) != get(new, &["workspace", "patch"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            reason: "Cargo.toml [workspace.patch] changed".to_string(),
        });
    }

    outcome
}

/// Walk a profile's `inherits` chain in `profiles` (using `fallback` for profiles missing on
/// the new side) and return true if `target` is `start` or any ancestor of `start`.
///
/// Cargo's built-in fallback chain (`test`→`dev`, `bench`→`release`) is encoded explicitly.
fn profile_in_active_chain(
    target: &str,
    start: &str,
    primary: &toml::value::Table,
    fallback: &toml::value::Table,
) -> bool {
    if target == start {
        return true;
    }

    // Built-in defaults — these inherit even if not declared.
    let implicit_inherits = match start {
        "test" => Some("dev"),
        "bench" => Some("release"),
        _ => None,
    };

    let mut current = start.to_string();
    let mut visited: std::collections::HashSet<String> = Default::default();
    visited.insert(current.clone());

    loop {
        let table = primary
            .get(&current)
            .and_then(as_table)
            .or_else(|| fallback.get(&current).and_then(as_table));

        let next = match table.and_then(|t| t.get("inherits")).and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => match implicit_inherits {
                Some(s) if current == start => s.to_string(),
                _ => return false,
            },
        };

        if next == target {
            return true;
        }
        if !visited.insert(next.clone()) {
            return false; // cycle
        }
        current = next;
    }
}

// ---------------------------------------------------------------------------------------------
// Dioxus.toml analysis
// ---------------------------------------------------------------------------------------------

fn analyze_dioxus_value(
    old: &toml::Value,
    new: &toml::Value,
    ctx: &AnalysisCtx,
) -> ConfigChangeOutcome {
    if old == new {
        return ConfigChangeOutcome::Ignore { note: None };
    }

    let mut outcome = ConfigChangeOutcome::Ignore { note: None };

    // ---- [application] — paths and identifiers compiled into the build ----
    let app_rebuild_keys: &[&str] = &[
        "name",
        "out_dir",
        "asset_dir",
        "public_dir",
        "tailwind_input",
        "tailwind_output",
        "ios_info_plist",
        "macos_info_plist",
        "ios_entitlements",
        "macos_entitlements",
        "android_manifest",
        "android_main_activity",
        "android_min_sdk_version",
    ];
    for key in app_rebuild_keys {
        if get(old, &["application", key]) != get(new, &["application", key]) {
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                reason: format!("Dioxus.toml [application].{key} changed"),
            });
        }
    }

    // ---- [web.app] — title and base_path get baked into HTML / WASM URLs ----
    if get(old, &["web", "app"]) != get(new, &["web", "app"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            reason: "Dioxus.toml [web.app] changed".to_string(),
        });
    }

    // ---- [web.proxy] — only consumed when devserver boots ----
    if get(old, &["web", "proxy"]) != get(new, &["web", "proxy"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::WarnRestart {
            reason: "Dioxus.toml [web.proxy] changed — restart `dx serve` to apply.".to_string(),
        });
    }

    // ---- [web.https] — TLS config initialized at boot ----
    if get(old, &["web", "https"]) != get(new, &["web", "https"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::WarnRestart {
            reason: "Dioxus.toml [web.https] changed — restart `dx serve` to apply.".to_string(),
        });
    }

    // ---- [web.watcher] — watcher mounted at startup; can't be re-mounted live (yet) ----
    if get(old, &["web", "watcher"]) != get(new, &["web", "watcher"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::WarnRestart {
            reason: "Dioxus.toml [web.watcher] changed — restart `dx serve` to apply."
                .to_string(),
        });
    }

    // ---- [web.resource] — injected into HTML at build time ----
    if get(old, &["web", "resource"]) != get(new, &["web", "resource"]) {
        outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
            reason: "Dioxus.toml [web.resource] changed".to_string(),
        });
    }

    // ---- [permissions] / [deep_links] / [background] ----
    for section in &["permissions", "deep_links", "background"] {
        if get(old, &[section]) != get(new, &[section]) {
            outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                reason: format!("Dioxus.toml [{section}] changed"),
            });
        }
    }

    // ---- Per-platform sections — only rebuild if they're the active platform ----
    let platform_sections = [
        ("ios", BundleFormat::Ios),
        ("android", BundleFormat::Android),
        ("macos", BundleFormat::MacOS),
        ("windows", BundleFormat::Windows),
        ("linux", BundleFormat::Linux),
    ];
    for (section, fmt) in platform_sections {
        if get(old, &[section]) != get(new, &[section]) {
            if ctx.active_bundle == fmt {
                outcome = outcome.escalate(ConfigChangeOutcome::FullRebuild {
                    reason: format!("Dioxus.toml [{section}] changed"),
                });
            } else {
                outcome = outcome.escalate(ConfigChangeOutcome::Ignore {
                    note: Some(format!(
                        "Saw change to [{section}] but active bundle is `{}` — ignoring.",
                        ctx.active_bundle
                    )),
                });
            }
        }
    }

    // [bundle], [components], [web.pre_compress], [web.wasm_opt] are intentionally NOT in
    // either rebuild or warn lists — they only matter for `dx bundle` / `dx components` / release
    // post-processing, none of which run during `dx serve`.

    outcome
}

// ---------------------------------------------------------------------------------------------
// TOML walking helpers
// ---------------------------------------------------------------------------------------------

fn get<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    let mut current = value;
    for key in path {
        let table = current.as_table()?;
        current = table.get(*key)?;
    }
    Some(current)
}

fn as_table(value: &toml::Value) -> Option<&toml::value::Table> {
    value.as_table()
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> toml::Value {
        toml::from_str(s).expect("valid toml")
    }

    fn ctx_dev_web() -> AnalysisCtx {
        AnalysisCtx {
            active_profile: "dev".to_string(),
            active_bundle: BundleFormat::Web,
        }
    }

    fn ctx_release_ios() -> AnalysisCtx {
        AnalysisCtx {
            active_profile: "release".to_string(),
            active_bundle: BundleFormat::Ios,
        }
    }

    fn assert_rebuild(outcome: &ConfigChangeOutcome) {
        assert!(
            matches!(outcome, ConfigChangeOutcome::FullRebuild { .. }),
            "expected FullRebuild, got {outcome:?}"
        );
    }

    fn assert_warn(outcome: &ConfigChangeOutcome) {
        assert!(
            matches!(outcome, ConfigChangeOutcome::WarnRestart { .. }),
            "expected WarnRestart, got {outcome:?}"
        );
    }

    fn assert_ignore(outcome: &ConfigChangeOutcome) {
        assert!(
            matches!(outcome, ConfigChangeOutcome::Ignore { .. }),
            "expected Ignore, got {outcome:?}"
        );
    }

    // ============================================================================
    // Cargo.toml — baseline + variants
    // ============================================================================

    const CARGO_BASELINE: &str = r#"
[package]
name = "demo"
version = "0.1.0"
edition = "2021"
description = "a demo crate"
license = "MIT"

[dependencies]
serde = "1"

[dev-dependencies]
proptest = "1"

[features]
default = ["foo"]
foo = []

[profile.dev]
opt-level = 0

[profile.release]
opt-level = 3
"#;

    #[test]
    fn cargo_identical_is_ignore() {
        let v = parse(CARGO_BASELINE);
        assert_ignore(&analyze_cargo_value(&v, &v, &ctx_dev_web()));
    }

    #[test]
    fn cargo_add_dependency_rebuilds() {
        let new = parse(&format!("{CARGO_BASELINE}\n[dependencies.tokio]\nversion = \"1\""));
        let outcome = analyze_cargo_value(&parse(CARGO_BASELINE), &new, &ctx_dev_web());
        assert_rebuild(&outcome);
    }

    #[test]
    fn cargo_bump_dep_version_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace(r#"serde = "1""#, r#"serde = "2""#));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_remove_dependency_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace(r#"serde = "1""#, ""));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_dev_dependency_rebuilds() {
        let new = parse(&format!(
            "{CARGO_BASELINE}\n[dev-dependencies.criterion]\nversion = \"0.5\""
        ));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_build_dependency_rebuilds() {
        let new = parse(&format!(
            "{CARGO_BASELINE}\n[build-dependencies]\ncc = \"1\""
        ));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_target_specific_dep_rebuilds() {
        let new = parse(&format!(
            "{CARGO_BASELINE}\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\""
        ));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_feature_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace("foo = []", "foo = []\nbar = []"));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_default_feature_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace(r#"default = ["foo"]"#, "default = []"));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_edition_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace(r#"edition = "2021""#, r#"edition = "2024""#));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_rust_version_rebuilds() {
        let spliced = CARGO_BASELINE.replace(
            r#"license = "MIT""#,
            "license = \"MIT\"\nrust-version = \"1.80\"",
        );
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&spliced),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_active_profile_rebuilds() {
        let new = parse(&CARGO_BASELINE.replace("opt-level = 0", "opt-level = 1"));
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &new,
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_inactive_profile_ignored_in_dev() {
        // Editing [profile.release] while serving in dev should NOT rebuild.
        let new = parse(&CARGO_BASELINE.replace("opt-level = 3", "opt-level = 2"));
        let outcome = analyze_cargo_value(&parse(CARGO_BASELINE), &new, &ctx_dev_web());
        assert_ignore(&outcome);
        if let ConfigChangeOutcome::Ignore { note: Some(n) } = &outcome {
            assert!(n.contains("[profile.release]"));
            assert!(n.contains("`dev`"));
        } else {
            panic!("expected Ignore-with-note, got {outcome:?}");
        }
    }

    #[test]
    fn cargo_change_inactive_profile_ignored_in_release() {
        // Editing [profile.dev] while serving in release should NOT rebuild.
        let new = parse(&CARGO_BASELINE.replace("opt-level = 0", "opt-level = 1"));
        let outcome = analyze_cargo_value(&parse(CARGO_BASELINE), &new, &ctx_release_ios());
        assert_ignore(&outcome);
    }

    #[test]
    fn cargo_change_inherited_profile_rebuilds() {
        // Active profile "android-dev" inherits from "dev". A change to [profile.dev] must
        // rebuild because it propagates through the inherits chain.
        let baseline = format!(
            "{CARGO_BASELINE}\n[profile.android-dev]\ninherits = \"dev\"\nopt-level = 1\n"
        );
        let modified = baseline.replacen("opt-level = 0", "opt-level = 1", 1);
        let ctx = AnalysisCtx {
            active_profile: "android-dev".to_string(),
            active_bundle: BundleFormat::Android,
        };
        let outcome = analyze_cargo_value(&parse(&baseline), &parse(&modified), &ctx);
        assert_rebuild(&outcome);
    }

    #[test]
    fn cargo_change_test_profile_with_implicit_dev_inheritance() {
        // `test` implicitly inherits from `dev`. Active = `test` → editing [profile.dev]
        // must rebuild even without an explicit `inherits` key.
        let new = parse(&CARGO_BASELINE.replace("opt-level = 0", "opt-level = 1"));
        let ctx = AnalysisCtx {
            active_profile: "test".to_string(),
            active_bundle: BundleFormat::Web,
        };
        assert_rebuild(&analyze_cargo_value(&parse(CARGO_BASELINE), &new, &ctx));
    }

    #[test]
    fn cargo_change_workspace_members_rebuilds_with_warn_note() {
        let baseline = r#"
[workspace]
members = ["a"]
"#;
        let modified = r#"
[workspace]
members = ["a", "b"]
"#;
        let outcome = analyze_cargo_value(&parse(baseline), &parse(modified), &ctx_dev_web());
        assert_rebuild(&outcome);
        if let ConfigChangeOutcome::FullRebuild { reason } = outcome {
            assert!(reason.contains("workspace") && reason.contains("hot-reloaded"));
        }
    }

    #[test]
    fn cargo_change_workspace_dependencies_rebuilds() {
        let baseline = r#"
[workspace]
members = ["a"]
[workspace.dependencies]
serde = "1"
"#;
        let modified = r#"
[workspace]
members = ["a"]
[workspace.dependencies]
serde = "2"
"#;
        assert_rebuild(&analyze_cargo_value(
            &parse(baseline),
            &parse(modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_patch_rebuilds() {
        let baseline = format!("{CARGO_BASELINE}");
        let modified = format!(
            "{CARGO_BASELINE}\n[patch.crates-io]\nserde = {{ git = \"https://github.com/serde-rs/serde\" }}\n"
        );
        assert_rebuild(&analyze_cargo_value(
            &parse(&baseline),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_lib_section_rebuilds() {
        let modified = format!("{CARGO_BASELINE}\n[lib]\nname = \"demo_lib\"\n");
        assert_rebuild(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_bin_path_rebuilds() {
        let baseline = format!(
            "{CARGO_BASELINE}\n[[bin]]\nname = \"demo\"\npath = \"src/main.rs\"\n"
        );
        let modified = format!(
            "{CARGO_BASELINE}\n[[bin]]\nname = \"demo\"\npath = \"src/bin.rs\"\n"
        );
        assert_rebuild(&analyze_cargo_value(
            &parse(&baseline),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_lints_section_ignored() {
        let modified = format!(
            "{CARGO_BASELINE}\n[lints.rust]\nunsafe_code = \"forbid\"\n"
        );
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_description_ignored() {
        let modified = CARGO_BASELINE.replace(r#"description = "a demo crate""#, r#"description = "an updated demo""#);
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_change_license_ignored() {
        let modified = CARGO_BASELINE.replace(r#"license = "MIT""#, r#"license = "Apache-2.0""#);
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_add_authors_ignored() {
        let modified = CARGO_BASELINE.replace(
            r#"license = "MIT""#,
            "license = \"MIT\"\nauthors = [\"jon\"]",
        );
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_reorder_keys_ignored() {
        let reordered = r#"
[package]
edition = "2021"
license = "MIT"
description = "a demo crate"
version = "0.1.0"
name = "demo"

[dev-dependencies]
proptest = "1"

[dependencies]
serde = "1"

[features]
foo = []
default = ["foo"]

[profile.release]
opt-level = 3

[profile.dev]
opt-level = 0
"#;
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(reordered),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_comment_only_change_ignored() {
        // Inserting a comment doesn't change the parsed Value at all.
        let modified = format!("# new comment\n{CARGO_BASELINE}");
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn cargo_whitespace_change_ignored() {
        let modified = CARGO_BASELINE.replace("\n\n", "\n\n\n\n");
        assert_ignore(&analyze_cargo_value(
            &parse(CARGO_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    // ============================================================================
    // Dioxus.toml — baseline + variants
    // ============================================================================

    const DIOXUS_BASELINE: &str = r#"
[application]
name = "demo"
public_dir = "public"

[web.app]
title = "demo"

[web.watcher]
watch_path = ["src"]
reload_html = false
index_on_404 = true

[bundle]
identifier = "com.example.demo"

[ios]
identifier = "com.example.demo.ios"

[android]
identifier = "com.example.demo.android"
"#;

    #[test]
    fn dioxus_identical_is_ignore() {
        let v = parse(DIOXUS_BASELINE);
        assert_ignore(&analyze_dioxus_value(&v, &v, &ctx_dev_web()));
    }

    #[test]
    fn dioxus_change_app_title_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(r#"title = "demo""#, r#"title = "renamed""#);
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_base_path_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(
            r#"title = "demo""#,
            "title = \"demo\"\nbase_path = \"/app\"",
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_public_dir_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(r#"public_dir = "public""#, r#"public_dir = "static""#);
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_tailwind_input_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(
            r#"public_dir = "public""#,
            "public_dir = \"public\"\ntailwind_input = \"src/input.css\"",
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_add_proxy_warns_restart() {
        let modified = format!(
            "{DIOXUS_BASELINE}\n[[web.proxy]]\nbackend = \"http://localhost:9999/api\"\n"
        );
        assert_warn(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_proxy_backend_warns_restart() {
        let baseline = format!(
            "{DIOXUS_BASELINE}\n[[web.proxy]]\nbackend = \"http://localhost:9999/api\"\n"
        );
        let modified = baseline.replace("9999", "8888");
        assert_warn(&analyze_dioxus_value(
            &parse(&baseline),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_enable_https_warns_restart() {
        let modified = format!("{DIOXUS_BASELINE}\n[web.https]\nenabled = true\n");
        assert_warn(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_watcher_paths_warns_restart() {
        let modified =
            DIOXUS_BASELINE.replace(r#"watch_path = ["src"]"#, r#"watch_path = ["src", "lib"]"#);
        assert_warn(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_add_dev_resource_script_rebuilds() {
        let modified = format!(
            "{DIOXUS_BASELINE}\n[web.resource.dev]\nscript = [\"http://example.com/x.js\"]\n"
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_bundle_identifier_ignored() {
        let modified = DIOXUS_BASELINE.replace(
            r#"identifier = "com.example.demo""#,
            r#"identifier = "com.example.renamed""#,
        );
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_components_section_ignored() {
        let modified = format!("{DIOXUS_BASELINE}\n[components]\ngit = \"https://example.com\"\n");
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_wasm_opt_level_ignored() {
        let modified = format!("{DIOXUS_BASELINE}\n[web.wasm_opt]\nlevel = \"3\"\n");
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_pre_compress_ignored() {
        let modified = format!("{DIOXUS_BASELINE}\n[web]\npre_compress = true\n");
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_change_ios_identifier_when_active_rebuilds() {
        let modified = DIOXUS_BASELINE.replace(
            r#"identifier = "com.example.demo.ios""#,
            r#"identifier = "com.example.renamed.ios""#,
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_release_ios(),
        ));
    }

    #[test]
    fn dioxus_change_ios_identifier_when_web_active_ignored() {
        let modified = DIOXUS_BASELINE.replace(
            r#"identifier = "com.example.demo.ios""#,
            r#"identifier = "com.example.renamed.ios""#,
        );
        let outcome = analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        );
        assert_ignore(&outcome);
        if let ConfigChangeOutcome::Ignore { note: Some(n) } = &outcome {
            assert!(n.contains("[ios]"));
        } else {
            panic!("expected Ignore-with-note, got {outcome:?}");
        }
    }

    #[test]
    fn dioxus_add_permission_rebuilds() {
        let modified = format!(
            "{DIOXUS_BASELINE}\n[permissions]\ncamera = {{ description = \"need camera\" }}\n"
        );
        assert_rebuild(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(&modified),
            &ctx_dev_web(),
        ));
    }

    #[test]
    fn dioxus_reorder_keys_ignored() {
        let reordered = r#"
[android]
identifier = "com.example.demo.android"

[ios]
identifier = "com.example.demo.ios"

[bundle]
identifier = "com.example.demo"

[web.watcher]
index_on_404 = true
reload_html = false
watch_path = ["src"]

[web.app]
title = "demo"

[application]
public_dir = "public"
name = "demo"
"#;
        assert_ignore(&analyze_dioxus_value(
            &parse(DIOXUS_BASELINE),
            &parse(reordered),
            &ctx_dev_web(),
        ));
    }

    // ============================================================================
    // Outcome escalation
    // ============================================================================

    #[test]
    fn escalate_full_rebuild_dominates_warn_restart() {
        let a = ConfigChangeOutcome::FullRebuild { reason: "a".into() };
        let b = ConfigChangeOutcome::WarnRestart { reason: "b".into() };
        assert_rebuild(&a.clone().escalate(b.clone()));
        assert_rebuild(&b.escalate(a));
    }

    #[test]
    fn escalate_warn_restart_dominates_ignore() {
        let a = ConfigChangeOutcome::WarnRestart { reason: "a".into() };
        let b = ConfigChangeOutcome::Ignore { note: None };
        assert_warn(&a.clone().escalate(b.clone()));
        assert_warn(&b.escalate(a));
    }

    // ============================================================================
    // ConfigWatcher integration (parse-error / round-trip)
    // ============================================================================

    #[test]
    fn watcher_returns_ignore_for_unchanged_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, CARGO_BASELINE).unwrap();

        let mut w = ConfigWatcher::new("dev".to_string(), BundleFormat::Web);
        w.seed_cargo(&path);

        let outcome = w.analyze_cargo_toml(&path);
        assert_ignore(&outcome);
    }

    #[test]
    fn watcher_detects_added_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, CARGO_BASELINE).unwrap();

        let mut w = ConfigWatcher::new("dev".to_string(), BundleFormat::Web);
        w.seed_cargo(&path);

        let modified = format!("{CARGO_BASELINE}\n[dependencies.tokio]\nversion = \"1\"\n");
        std::fs::write(&path, &modified).unwrap();

        let outcome = w.analyze_cargo_toml(&path);
        assert_rebuild(&outcome);
    }

    #[test]
    fn watcher_returns_ignore_with_note_on_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, CARGO_BASELINE).unwrap();

        let mut w = ConfigWatcher::new("dev".to_string(), BundleFormat::Web);
        w.seed_cargo(&path);

        // Mid-edit garbage
        std::fs::write(&path, "[package\nthis is not valid").unwrap();
        let outcome = w.analyze_cargo_toml(&path);
        assert_ignore(&outcome);
        if let ConfigChangeOutcome::Ignore { note: Some(n) } = &outcome {
            assert!(n.contains("parse failed"));
        } else {
            panic!("expected Ignore-with-note, got {outcome:?}");
        }

        // Subsequent valid save still diffs against the *seeded* baseline.
        let modified = format!("{CARGO_BASELINE}\n[dependencies.tokio]\nversion = \"1\"\n");
        std::fs::write(&path, &modified).unwrap();
        assert_rebuild(&w.analyze_cargo_toml(&path));
    }

    #[test]
    fn watcher_no_repeat_rebuild_for_same_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, CARGO_BASELINE).unwrap();

        let mut w = ConfigWatcher::new("dev".to_string(), BundleFormat::Web);
        w.seed_cargo(&path);

        let modified = format!("{CARGO_BASELINE}\n[dependencies.tokio]\nversion = \"1\"\n");
        std::fs::write(&path, &modified).unwrap();
        assert_rebuild(&w.analyze_cargo_toml(&path));

        // Second analyze call against the *same* file content should be Ignore — the snapshot
        // was updated by the first call, so we don't loop on the same edit.
        assert_ignore(&w.analyze_cargo_toml(&path));
    }
}
