mod ios;
mod linux;
mod macos;
mod tools;
mod updater;
mod windows;

use crate::PackageType;
use crate::{BuildRequest, DebianSettings, MacOsSettings, WindowsSettings};
use anyhow::Context;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tools::ResolvedTools;

/// A completed bundle with its output paths.
#[derive(Debug)]
pub(crate) struct Bundle {
    pub package_type: PackageType,
    pub bundle_paths: Vec<PathBuf>,
}

/// BundleContext wraps a BuildRequest and provides the settings API
/// that bundle format modules need. This is the adapter between
/// Dioxus build infrastructure and the individual bundler implementations.
pub(crate) struct BundleContext<'a> {
    pub(crate) build: &'a BuildRequest,

    pub(crate) package_types: Vec<PackageType>,

    /// Pre-computed resource map: source path -> target path in bundle
    pub(crate) resources_map: HashMap<String, String>,

    /// Pre-resolved tool paths (NSIS, WiX, linuxdeploy, WebView2).
    pub(crate) tools: ResolvedTools,
}

impl<'a> BundleContext<'a> {
    /// Create a new BundleContext from a BuildRequest and optional package types.
    pub(crate) async fn new(
        build: &'a BuildRequest,
        package_types: &Option<Vec<PackageType>>,
    ) -> Result<Self> {
        let package_types = package_types.clone().unwrap_or_default();

        // Build the resources map from assets + config resources
        let mut resources_map = HashMap::new();

        let asset_dir = build.bundle_asset_dir();
        if asset_dir.exists() {
            for entry in walkdir::WalkDir::new(&asset_dir) {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let old = path
                        .canonicalize()
                        .with_context(|| format!("Failed to canonicalize {entry:?}"))?;
                    let new =
                        PathBuf::from("assets").join(path.strip_prefix(&asset_dir).unwrap_or(path));
                    resources_map.insert(old.display().to_string(), new.display().to_string());
                }
            }
        }

        // Merge in any custom resources from config
        if let Some(resources) = &build.config.bundle.resources {
            for resource_path in resources {
                resources_map.insert(resource_path.clone(), String::new());
            }
        }

        let tools_dir = crate::Workspace::tools_dir();
        let _ = std::fs::create_dir_all(&tools_dir);

        let arch = {
            let target = build.triple.to_string();
            if target.starts_with("x86_64") {
                Arch::X86_64
            } else if target.starts_with('i') {
                Arch::X86
            } else if target.starts_with("arm") && target.ends_with("hf") {
                Arch::Armhf
            } else if target.starts_with("arm") {
                Arch::Armel
            } else if target.starts_with("aarch64") {
                Arch::AArch64
            } else if target.starts_with("riscv64") {
                Arch::Riscv64
            } else if target.starts_with("universal") {
                Arch::Universal
            } else if cfg!(target_arch = "x86_64") {
                Arch::X86_64
            } else if cfg!(target_arch = "aarch64") {
                Arch::AArch64
            } else {
                Arch::X86_64
            }
        };

        let windows_settings = build.config.bundle.windows.clone().unwrap_or_default();
        let tools =
            tools::resolve_tools(&tools_dir, &package_types, &windows_settings, arch).await?;

        Ok(Self {
            build,
            package_types,
            resources_map,
            tools,
        })
    }

    /// Bundle the project for every configured package type, in dependency order.
    ///
    /// This is the orchestration entrypoint for native packaging. It owns the
    /// sequencing rules between bundle formats, but delegates the actual format
    /// implementation to the per-platform `bundle_*` methods on `BundleContext`.
    ///
    /// The method performs the following high-level workflow:
    /// 1. Read the resolved package types from the immutable context.
    /// 2. Sort them so prerequisite artifacts are built before dependents.
    ///    In practice, this means raw distributable formats such as `.app`, `.deb`,
    ///    `.rpm`, `.AppImage`, `.msi`, `.exe`, `.apk`, and `.aab` run before
    ///    dependent archive formats like `.ipa`/`.dmg`, and `Updater` always runs last.
    /// 3. Dispatch to the top-level format-specific bundling method for each package
    ///    type and collect the artifact paths it returns.
    /// 4. Reuse outputs where formats depend on each other. For example, DMG creation
    ///    may synthesize a `.app` bundle and return it alongside the `.dmg`.
    /// 5. Remove intermediate macOS `.app` bundles when they were only needed as an
    ///    implementation detail of `.dmg` or updater generation.
    ///
    /// The returned [`Bundle`] values are the canonical record of what was emitted by
    /// the bundling pass. Later steps such as updater packaging rely on these paths
    /// rather than rediscovering artifacts on disk.
    pub(crate) async fn bundle_project(&self) -> Result<Vec<Bundle>> {
        let mut package_types = self.package_types();

        // Sort so dependencies come first (e.g. .app before .dmg)
        package_types.sort_by_key(|ptype| match ptype {
            PackageType::MacOsBundle
            | PackageType::IosApp
            | PackageType::WindowsMsi
            | PackageType::Nsis
            | PackageType::Deb
            | PackageType::Rpm
            | PackageType::AppImage
            | PackageType::Apk
            | PackageType::Aab => 0,
            PackageType::Ipa | PackageType::Dmg => 1,
            PackageType::Updater => 2,
        });

        let mut bundles = Vec::<Bundle>::new();

        for package_type in &package_types {
            // Skip if already built (e.g. DMG already built .app)
            if bundles.iter().any(|b| b.package_type == *package_type) {
                continue;
            }

            let bundle_paths = match package_type {
                PackageType::MacOsBundle => self.bundle_macos_app().await?,
                PackageType::IosApp => self.bundle_ios_app().await?,
                PackageType::Ipa => self.bundle_ios_ipa(&bundles).await?.ipa,
                PackageType::Dmg => {
                    let bundled = self.bundle_macos_dmg(&bundles).await?;
                    if !bundled.app.is_empty() {
                        bundles.push(Bundle {
                            package_type: PackageType::MacOsBundle,
                            bundle_paths: bundled.app,
                        });
                    }
                    bundled.dmg
                }
                PackageType::Deb => self.bundle_linux_deb().await?,
                PackageType::Rpm => self.bundle_linux_rpm().await?,
                PackageType::AppImage => self.bundle_linux_appimage().await?,
                PackageType::WindowsMsi => self.bundle_windows_msi().await?,
                PackageType::Nsis => self.bundle_windows_nsis().await?,
                PackageType::Updater => self.bundle_updater(&bundles).await?,
                PackageType::Apk | PackageType::Aab => self.bundle_android(*package_type).await?,
            };

            bundles.push(Bundle {
                package_type: *package_type,
                bundle_paths,
            });
        }

        // On macOS, clean up .app if only building dmg or updater
        if !package_types.contains(&PackageType::MacOsBundle) {
            if let Some(idx) = bundles
                .iter()
                .position(|b| b.package_type == PackageType::MacOsBundle)
            {
                let app_bundle = bundles.remove(idx);
                for path in &app_bundle.bundle_paths {
                    tracing::info!("Cleaning up intermediate .app: {}", path.display());
                    if path.is_dir() {
                        let _ = std::fs::remove_dir_all(path);
                    } else {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }

        for bundle in &bundles {
            for path in &bundle.bundle_paths {
                tracing::info!("Bundled: {}", path.display());
            }
        }

        Ok(bundles)
    }

    /// The package types to bundle.
    pub(crate) fn package_types(&self) -> Vec<PackageType> {
        self.package_types.clone()
    }

    /// The product name (PascalCase).
    pub(crate) fn product_name(&self) -> String {
        self.build.bundled_app_name()
    }

    /// The bundle identifier (e.g. "com.example.app").
    pub(crate) fn bundle_identifier(&self) -> String {
        self.build.bundle_identifier()
    }

    /// The publisher name.
    pub(crate) fn publisher(&self) -> Option<&str> {
        self.build.config.bundle.publisher.as_deref()
    }

    /// The version string from Cargo.toml.
    pub(crate) fn version_string(&self) -> String {
        self.build.package().version.to_string()
    }

    /// The short description.
    pub(crate) fn short_description(&self) -> String {
        self.build
            .config
            .bundle
            .short_description
            .clone()
            .or_else(|| self.build.package().description.clone())
            .unwrap_or_default()
    }

    /// The long description.
    pub(crate) fn long_description(&self) -> Option<&str> {
        self.build.config.bundle.long_description.as_deref()
    }

    /// The copyright string.
    pub(crate) fn copyright_string(&self) -> Option<&str> {
        self.build.config.bundle.copyright.as_deref()
    }

    /// The authors list.
    pub(crate) fn authors(&self) -> &[String] {
        &self.build.package().authors
    }

    /// Authors as a comma-separated string.
    pub(crate) fn authors_comma_separated(&self) -> Option<String> {
        let names = self.authors();
        if names.is_empty() {
            None
        } else {
            Some(names.join(", "))
        }
    }

    /// The homepage URL.
    pub(crate) fn homepage_url(&self) -> Option<String> {
        self.build
            .package()
            .homepage
            .as_ref()
            .filter(|hp| !hp.is_empty())
            .cloned()
    }

    /// The license string.
    pub(crate) fn license(&self) -> Option<&str> {
        self.build.package().license.as_deref()
    }

    /// The app category string.
    pub(crate) fn app_category(&self) -> Option<&str> {
        self.build.config.bundle.category.as_deref()
    }

    /// The main binary name (without extension).
    pub(crate) fn main_binary_name(&self) -> &str {
        self.build.executable_name()
    }

    /// The path to the main binary.
    pub(crate) fn main_binary_path(&self) -> PathBuf {
        self.build.main_exe()
    }

    /// The output directory for bundles.
    pub(crate) fn project_out_directory(&self) -> PathBuf {
        self.build.bundle_dir(self.build.bundle)
    }

    /// The target triple string.
    pub(crate) fn target(&self) -> String {
        self.build.triple.to_string()
    }

    /// The binary architecture.
    pub(crate) fn binary_arch(&self) -> Arch {
        let target = self.target();
        if target.starts_with("x86_64") {
            Arch::X86_64
        } else if target.starts_with('i') {
            Arch::X86
        } else if target.starts_with("arm") && target.ends_with("hf") {
            Arch::Armhf
        } else if target.starts_with("arm") {
            Arch::Armel
        } else if target.starts_with("aarch64") {
            Arch::AArch64
        } else if target.starts_with("riscv64") {
            Arch::Riscv64
        } else if target.starts_with("universal") {
            Arch::Universal
        } else {
            // Default to the host arch
            if cfg!(target_arch = "x86_64") {
                Arch::X86_64
            } else if cfg!(target_arch = "aarch64") {
                Arch::AArch64
            } else {
                Arch::X86_64
            }
        }
    }

    /// Icon file paths from config, canonicalized relative to the crate dir.
    pub(crate) fn icon_files(&self) -> Result<Vec<PathBuf>> {
        let mut icons = Vec::new();
        if let Some(icon_paths) = &self.build.config.bundle.icon {
            for icon in icon_paths {
                let icon_path = self
                    .build
                    .crate_dir()
                    .join(icon)
                    .canonicalize()
                    .with_context(|| format!("Failed to canonicalize icon path: {icon}"))?;
                icons.push(icon_path);
            }
        }
        Ok(icons)
    }

    /// Copy resources to the given path.
    pub(crate) fn copy_resources(&self, dest: &Path) -> Result<()> {
        for (src, target) in &self.resources_map {
            let src_path = PathBuf::from(src);
            if !src_path.exists() {
                tracing::warn!("Resource not found: {src}");
                continue;
            }
            let dest_path = if target.is_empty() {
                dest.join(src_path.file_name().unwrap_or_default())
            } else {
                dest.join(target)
            };
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src_path, &dest_path).with_context(|| {
                format!("Failed to copy resource {src} -> {}", dest_path.display())
            })?;
        }
        Ok(())
    }

    /// Copy external binaries to the given path.
    pub(crate) fn copy_external_binaries(&self, dest: &Path) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        if let Some(bins) = &self.build.config.bundle.external_bin {
            let target = self.target();
            for bin in bins {
                let src = PathBuf::from(format!("{bin}-{target}"));
                if src.exists() {
                    let dest_name = src
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .replace(&format!("-{target}"), "");
                    let dest_path = dest.join(dest_name);
                    std::fs::copy(&src, &dest_path)?;
                    paths.push(dest_path);
                }
            }
        }
        Ok(paths)
    }

    /// The crate directory (where Cargo.toml is).
    pub(crate) fn crate_dir(&self) -> PathBuf {
        self.build.crate_dir()
    }

    /// Debian settings from config.
    pub(crate) fn deb(&self) -> DebianSettings {
        self.build.config.bundle.deb.clone().unwrap_or_default()
    }

    /// macOS settings from config.
    pub(crate) fn macos(&self) -> MacOsSettings {
        self.build.config.bundle.macos.clone().unwrap_or_default()
    }

    /// Windows settings from config.
    pub(crate) fn windows(&self) -> WindowsSettings {
        self.build.config.bundle.windows.clone().unwrap_or_default()
    }
}

/// The architecture of the target binary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Arch {
    X86_64,
    X86,
    AArch64,
    Armhf,
    Armel,
    Riscv64,
    Universal,
}

impl Arch {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::X86 => "x86",
            Arch::AArch64 => "aarch64",
            Arch::Armhf => "armhf",
            Arch::Armel => "armel",
            Arch::Riscv64 => "riscv64",
            Arch::Universal => "universal",
        }
    }

    pub(crate) fn deb_arch(&self) -> &'static str {
        match self {
            Arch::X86_64 => "amd64",
            Arch::X86 => "i386",
            Arch::AArch64 => "arm64",
            Arch::Armhf => "armhf",
            Arch::Armel => "armel",
            Arch::Riscv64 => "riscv64",
            Arch::Universal => "all",
        }
    }

    pub(crate) fn rpm_arch(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::X86 => "i686",
            Arch::AArch64 => "aarch64",
            Arch::Armhf => "armv7hl",
            Arch::Armel => "armv6l",
            Arch::Riscv64 => "riscv64",
            Arch::Universal => "noarch",
        }
    }

    pub(crate) fn appimage_arch(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::X86 => "i386",
            Arch::AArch64 => "aarch64",
            Arch::Armhf => "armhf",
            Arch::Armel => "armel",
            Arch::Riscv64 => "riscv64",
            Arch::Universal => "x86_64",
        }
    }

    pub(crate) fn windows_arch(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x64",
            Arch::X86 => "x86",
            Arch::AArch64 => "arm64",
            _ => "x64",
        }
    }

    pub(crate) fn wix_arch(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x64",
            Arch::X86 => "x86",
            Arch::AArch64 => "arm64",
            _ => "x64",
        }
    }

    pub(crate) fn wix_program_files_folder(&self) -> &'static str {
        match self {
            Arch::X86 => "ProgramFilesFolder",
            Arch::X86_64 | Arch::AArch64 => "ProgramFiles64Folder",
            _ => "ProgramFiles64Folder",
        }
    }

    pub(crate) fn linuxdeploy_arch(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::X86 => "i386",
            Arch::AArch64 => "aarch64",
            Arch::Armhf | Arch::Armel => "armhf",
            Arch::Riscv64 | Arch::Universal => "x86_64",
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

use std::str::FromStr;

/// The possible app categories.
/// Corresponds to `LSApplicationCategoryType` on macOS and the GNOME desktop categories on Debian.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppCategory {
    Business,
    DeveloperTool,
    Education,
    Entertainment,
    Finance,
    Game,
    ActionGame,
    AdventureGame,
    ArcadeGame,
    BoardGame,
    CardGame,
    CasinoGame,
    DiceGame,
    EducationalGame,
    FamilyGame,
    KidsGame,
    MusicGame,
    PuzzleGame,
    RacingGame,
    RolePlayingGame,
    SimulationGame,
    SportsGame,
    StrategyGame,
    TriviaGame,
    WordGame,
    GraphicsAndDesign,
    HealthcareAndFitness,
    Lifestyle,
    Medical,
    Music,
    News,
    Photography,
    Productivity,
    Reference,
    SocialNetworking,
    Sports,
    Travel,
    Utility,
    Video,
    Weather,
}

impl FromStr for AppCategory {
    type Err = String;

    fn from_str(input: &str) -> Result<AppCategory, Self::Err> {
        const MACOS_APP_CATEGORY_PREFIX: &str = "public.app-category.";

        let mut input = input.to_ascii_lowercase();
        if input.starts_with(MACOS_APP_CATEGORY_PREFIX) {
            input = input
                .split_at(MACOS_APP_CATEGORY_PREFIX.len())
                .1
                .to_string();
        }
        input = input.replace(' ', "");
        input = input.replace('-', "");

        for &(string, category) in CATEGORY_STRINGS.iter() {
            if input == string {
                return Ok(category);
            }
        }
        Err(format!("Unknown app category: {input}"))
    }
}

impl AppCategory {
    /// Map to closest set of Freedesktop categories.
    pub(crate) fn freedesktop_categories(self) -> &'static str {
        match &self {
            AppCategory::Business => "Office;",
            AppCategory::DeveloperTool => "Development;",
            AppCategory::Education => "Education;",
            AppCategory::Entertainment => "Network;",
            AppCategory::Finance => "Office;Finance;",
            AppCategory::Game => "Game;",
            AppCategory::ActionGame => "Game;ActionGame;",
            AppCategory::AdventureGame => "Game;AdventureGame;",
            AppCategory::ArcadeGame => "Game;ArcadeGame;",
            AppCategory::BoardGame => "Game;BoardGame;",
            AppCategory::CardGame => "Game;CardGame;",
            AppCategory::CasinoGame => "Game;",
            AppCategory::DiceGame => "Game;",
            AppCategory::EducationalGame => "Game;Education;",
            AppCategory::FamilyGame => "Game;",
            AppCategory::KidsGame => "Game;KidsGame;",
            AppCategory::MusicGame => "Game;",
            AppCategory::PuzzleGame => "Game;LogicGame;",
            AppCategory::RacingGame => "Game;",
            AppCategory::RolePlayingGame => "Game;RolePlaying;",
            AppCategory::SimulationGame => "Game;Simulation;",
            AppCategory::SportsGame => "Game;SportsGame;",
            AppCategory::StrategyGame => "Game;StrategyGame;",
            AppCategory::TriviaGame => "Game;",
            AppCategory::WordGame => "Game;",
            AppCategory::GraphicsAndDesign => "Graphics;",
            AppCategory::HealthcareAndFitness => "Science;",
            AppCategory::Lifestyle => "Education;",
            AppCategory::Medical => "Science;MedicalSoftware;",
            AppCategory::Music => "AudioVideo;Audio;Music;",
            AppCategory::News => "Network;News;",
            AppCategory::Photography => "Graphics;Photography;",
            AppCategory::Productivity => "Office;",
            AppCategory::Reference => "Education;",
            AppCategory::SocialNetworking => "Network;",
            AppCategory::Sports => "Education;Sports;",
            AppCategory::Travel => "Education;",
            AppCategory::Utility => "Utility;",
            AppCategory::Video => "AudioVideo;Video;",
            AppCategory::Weather => "Science;",
        }
    }

    /// Map to macOS LSApplicationCategoryType.
    pub(crate) fn macos_application_category_type(self) -> &'static str {
        match &self {
            AppCategory::Business => "public.app-category.business",
            AppCategory::DeveloperTool => "public.app-category.developer-tools",
            AppCategory::Education => "public.app-category.education",
            AppCategory::Entertainment => "public.app-category.entertainment",
            AppCategory::Finance => "public.app-category.finance",
            AppCategory::Game => "public.app-category.games",
            AppCategory::ActionGame => "public.app-category.action-games",
            AppCategory::AdventureGame => "public.app-category.adventure-games",
            AppCategory::ArcadeGame => "public.app-category.arcade-games",
            AppCategory::BoardGame => "public.app-category.board-games",
            AppCategory::CardGame => "public.app-category.card-games",
            AppCategory::CasinoGame => "public.app-category.casino-games",
            AppCategory::DiceGame => "public.app-category.dice-games",
            AppCategory::EducationalGame => "public.app-category.educational-games",
            AppCategory::FamilyGame => "public.app-category.family-games",
            AppCategory::KidsGame => "public.app-category.kids-games",
            AppCategory::MusicGame => "public.app-category.music-games",
            AppCategory::PuzzleGame => "public.app-category.puzzle-games",
            AppCategory::RacingGame => "public.app-category.racing-games",
            AppCategory::RolePlayingGame => "public.app-category.role-playing-games",
            AppCategory::SimulationGame => "public.app-category.simulation-games",
            AppCategory::SportsGame => "public.app-category.sports-games",
            AppCategory::StrategyGame => "public.app-category.strategy-games",
            AppCategory::TriviaGame => "public.app-category.trivia-games",
            AppCategory::WordGame => "public.app-category.word-games",
            AppCategory::GraphicsAndDesign => "public.app-category.graphics-design",
            AppCategory::HealthcareAndFitness => "public.app-category.healthcare-fitness",
            AppCategory::Lifestyle => "public.app-category.lifestyle",
            AppCategory::Medical => "public.app-category.medical",
            AppCategory::Music => "public.app-category.music",
            AppCategory::News => "public.app-category.news",
            AppCategory::Photography => "public.app-category.photography",
            AppCategory::Productivity => "public.app-category.productivity",
            AppCategory::Reference => "public.app-category.reference",
            AppCategory::SocialNetworking => "public.app-category.social-networking",
            AppCategory::Sports => "public.app-category.sports",
            AppCategory::Travel => "public.app-category.travel",
            AppCategory::Utility => "public.app-category.utilities",
            AppCategory::Video => "public.app-category.video",
            AppCategory::Weather => "public.app-category.weather",
        }
    }
}

const CATEGORY_STRINGS: &[(&str, AppCategory)] = &[
    ("actiongame", AppCategory::ActionGame),
    ("actiongames", AppCategory::ActionGame),
    ("adventuregame", AppCategory::AdventureGame),
    ("adventuregames", AppCategory::AdventureGame),
    ("arcadegame", AppCategory::ArcadeGame),
    ("arcadegames", AppCategory::ArcadeGame),
    ("boardgame", AppCategory::BoardGame),
    ("boardgames", AppCategory::BoardGame),
    ("business", AppCategory::Business),
    ("cardgame", AppCategory::CardGame),
    ("cardgames", AppCategory::CardGame),
    ("casinogame", AppCategory::CasinoGame),
    ("casinogames", AppCategory::CasinoGame),
    ("developer", AppCategory::DeveloperTool),
    ("developertool", AppCategory::DeveloperTool),
    ("developertools", AppCategory::DeveloperTool),
    ("development", AppCategory::DeveloperTool),
    ("dicegame", AppCategory::DiceGame),
    ("dicegames", AppCategory::DiceGame),
    ("education", AppCategory::Education),
    ("educationalgame", AppCategory::EducationalGame),
    ("educationalgames", AppCategory::EducationalGame),
    ("entertainment", AppCategory::Entertainment),
    ("familygame", AppCategory::FamilyGame),
    ("familygames", AppCategory::FamilyGame),
    ("finance", AppCategory::Finance),
    ("fitness", AppCategory::HealthcareAndFitness),
    ("game", AppCategory::Game),
    ("games", AppCategory::Game),
    ("graphicdesign", AppCategory::GraphicsAndDesign),
    ("graphicsanddesign", AppCategory::GraphicsAndDesign),
    ("graphicsdesign", AppCategory::GraphicsAndDesign),
    ("healthcareandfitness", AppCategory::HealthcareAndFitness),
    ("healthcarefitness", AppCategory::HealthcareAndFitness),
    ("kidsgame", AppCategory::KidsGame),
    ("kidsgames", AppCategory::KidsGame),
    ("lifestyle", AppCategory::Lifestyle),
    ("logicgame", AppCategory::PuzzleGame),
    ("medical", AppCategory::Medical),
    ("music", AppCategory::Music),
    ("musicgame", AppCategory::MusicGame),
    ("musicgames", AppCategory::MusicGame),
    ("news", AppCategory::News),
    ("photography", AppCategory::Photography),
    ("productivity", AppCategory::Productivity),
    ("puzzlegame", AppCategory::PuzzleGame),
    ("puzzlegames", AppCategory::PuzzleGame),
    ("racinggame", AppCategory::RacingGame),
    ("racinggames", AppCategory::RacingGame),
    ("reference", AppCategory::Reference),
    ("roleplaying", AppCategory::RolePlayingGame),
    ("roleplayinggame", AppCategory::RolePlayingGame),
    ("roleplayinggames", AppCategory::RolePlayingGame),
    ("rpg", AppCategory::RolePlayingGame),
    ("simulationgame", AppCategory::SimulationGame),
    ("simulationgames", AppCategory::SimulationGame),
    ("socialnetwork", AppCategory::SocialNetworking),
    ("socialnetworking", AppCategory::SocialNetworking),
    ("sports", AppCategory::Sports),
    ("sportsgame", AppCategory::SportsGame),
    ("sportsgames", AppCategory::SportsGame),
    ("strategygame", AppCategory::StrategyGame),
    ("strategygames", AppCategory::StrategyGame),
    ("travel", AppCategory::Travel),
    ("triviagame", AppCategory::TriviaGame),
    ("triviagames", AppCategory::TriviaGame),
    ("utilities", AppCategory::Utility),
    ("utility", AppCategory::Utility),
    ("video", AppCategory::Video),
    ("weather", AppCategory::Weather),
    ("wordgame", AppCategory::WordGame),
    ("wordgames", AppCategory::WordGame),
];

/// Recursively copy a directory tree.
///
/// Preserves symlinks on unix targets and falls back to copying link targets on non-unix.
pub(crate) fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &dest_path)?;
        } else if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let target = std::fs::read_link(&source_path)?;
                std::os::unix::fs::symlink(&target, &dest_path)?;
            }

            #[cfg(not(unix))]
            {
                std::fs::copy(&source_path, &dest_path)?;
            }
        } else {
            std::fs::copy(&source_path, &dest_path)?;
        }
    }

    Ok(())
}

/// Recursively zip a directory tree while preserving relative paths and Unix modes.
pub(crate) fn zip_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    use std::fs::File;
    use std::io::{Read, Write};

    let file =
        File::create(dest).with_context(|| format!("Failed to create {}", dest.display()))?;
    let mut zip = zip::ZipWriter::new(file);

    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(src)
            .with_context(|| format!("Failed to strip prefix for {}", path.display()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let archive_path = relative.to_string_lossy().replace('\\', "/");
        let metadata = entry.metadata()?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };
        #[cfg(not(unix))]
        let mode = if metadata.is_dir() { 0o755 } else { 0o644 };

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(mode);

        if metadata.is_dir() {
            zip.add_directory(format!("{archive_path}/"), options)?;
            continue;
        }

        zip.start_file(&archive_path, options)?;
        let mut src_file = File::open(path)?;
        let mut buffer = Vec::new();
        src_file.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
    }

    zip.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::zip_dir_recursive;

    #[test]
    fn zip_dir_preserves_layout_and_permissions() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let payload = src.join("Payload/Test.app");
        std::fs::create_dir_all(&payload).unwrap();

        let exec = payload.join("runner");
        std::fs::write(&exec, "hello").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&exec, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let zip_path = temp.path().join("bundle.zip");
        zip_dir_recursive(&src, &zip_path).unwrap();

        let file = std::fs::File::open(&zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        let entry = archive.by_name("Payload/Test.app/runner").unwrap();
        let expected_mode = if cfg!(unix) { 0o755 } else { 0o644 };
        assert_eq!(
            entry.unix_mode().map(|mode| mode & 0o777),
            Some(expected_mode)
        );
    }
}
