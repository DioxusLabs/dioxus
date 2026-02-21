Plan: Remove tauri-bundle, Inline Bundling Logic
Context
The Dioxus CLI currently depends on tauri-bundler, tauri-utils, and tauri-macos-sign for desktop bundling (macOS .app/.dmg, Windows MSI/NSIS, Linux .deb/.rpm/.AppImage). These crates introduce breaking changes in patch releases and contain Tauri-specific logic we don't need. Dioxus already handles .app creation, Android/iOS bundling, and codesigning natively — only desktop dx bundle flows through tauri-bundler. This plan removes all three tauri dependencies by inlining the bundling logic into a new bundler/ module.

What Changes
1. New module: packages/cli/src/bundler/

bundler/
  mod.rs            -- bundle_project() orchestrator, Bundle result type
  context.rs        -- BundleContext wrapping &BuildRequest (mirrors tauri Settings API)
  tools.rs          -- Tool downloading (WiX, NSIS, linuxdeploy) to ~/.cache/dioxus/
  category.rs       -- AppCategory enum (ported from tauri-bundler)
  updater.rs        -- Zip/tar of bundle artifacts
  macos/
    mod.rs
    app.rs          -- .app enrichment (ICNS, frameworks, external bins) atop existing build output
    dmg.rs          -- DMG via hdiutil + AppleScript
    icon.rs         -- PNG → ICNS conversion
    sign.rs         -- codesign + notarization (inlined from tauri-macos-sign)
  linux/
    mod.rs
    debian.rs       -- .deb (pure Rust: ar + tar + flate2)
    rpm.rs          -- .rpm (rpm crate)
    appimage.rs     -- AppImage via linuxdeploy
    freedesktop.rs  -- .desktop file + icon hierarchy
  windows/
    mod.rs
    msi.rs          -- WiX MSI (candle + light)
    nsis.rs         -- NSIS installer (makensis)
    sign.rs         -- signtool.exe / custom sign command
    util.rs         -- WebView2 bootstrapper downloads
Templates (.nsi, .wxs, .desktop, .applescript) go in packages/cli/assets/bundler/.

2. BundleContext adapter (bundler/context.rs)
Wraps &BuildRequest and exposes methods matching tauri-bundler's Settings API (product_name(), bundle_identifier(), icon_files(), macos(), deb(), etc.). This makes porting format modules straightforward — swap the import, keep method names.

3. Modify cli/bundle.rs
Replace:


use tauri_bundler::{BundleBinary, BundleSettings, PackageSettings, SettingsBuilder};
let bundles = tauri_bundler::bundle::bundle_project(&settings)
With:


use crate::bundler::{BundleContext, bundle_project};
let ctx = BundleContext::new(build, &package_types)?;
let bundles = bundle_project(&ctx)?;
4. Delete bundle_utils.rs
The entire conversion layer (Dioxus types → tauri types) becomes unnecessary since the new bundler uses Dioxus config types directly.

5. Config types stay the same
config/bundle.rs types (BundleConfig, DebianSettings, WixSettings, MacOsSettings, WindowsSettings, NsisSettings, PackageType, etc.) are unchanged — the user-facing API is preserved.

6. macOS .app strategy
Dioxus already creates .app during build. The new bundler/macos/app.rs enriches the existing .app rather than creating from scratch:

Generate .icns icon → Contents/Resources/
Copy frameworks from MacOsSettings::frameworks
Copy custom files from MacOsSettings::files
Copy external binaries
Re-codesign + notarize
7. Dependency changes
Remove from workspace + cli Cargo.toml:

tauri-bundler, tauri-utils, tauri-macos-sign
Add (platform-gated where appropriate):

icns = "0.3" — ICNS generation (macOS; avoids another tauri crate)
image = "0.25" — icon dimension reading
rpm = "0.16" — RPM building (Linux)
md5 = "0.8" — deb checksums (Linux)
sha1 = "0.10" / sha2 = "0.10" — tool download verification
hex = "0.4" — hash encoding
zip = "4" — updater bundles
Already present (no change): ar, flate2, tar, handlebars, uuid, plist, reqwest, walkdir, which, regex, tempfile

8. Tool downloading (bundler/tools.rs)
Cache location: ~/.cache/dioxus/ (not ~/.cache/tauri/)
Use existing reqwest for HTTP
SHA hash verification before extraction
Functions: ensure_wix(), ensure_nsis(), ensure_linuxdeploy()
Same download URLs from tauri's GitHub releases (public), can fork later
Implementation Order
Each phase is independently testable:

Scaffold + macOS .app — Module structure, BundleContext, app.rs (enrich existing .app), icon.rs, sign.rs (inline codesign+notarize). Wire up cli/bundle.rs. Test: dx bundle --package-types macos
macOS DMG — dmg.rs with hdiutil + AppleScript. Test: dx bundle --package-types dmg
Linux .deb — debian.rs + freedesktop.rs (pure Rust, no external tools). Test: dx bundle --package-types deb
Linux .rpm + AppImage — rpm.rs, appimage.rs, tools.rs (linuxdeploy download). Test: dx bundle --package-types rpm / appimage
Windows NSIS — nsis.rs, windows/sign.rs, tools.rs (NSIS download). Test: dx bundle --package-types nsis
Windows MSI — msi.rs, tools.rs (WiX download). Test: dx bundle --package-types msi
Updater + cleanup — updater.rs, category.rs, delete bundle_utils.rs, remove tauri deps from Cargo.toml
Key Files to Modify
cli/bundle.rs — Replace tauri_bundler calls with new bundler module
bundle_utils.rs — Delete entirely
Cargo.toml (workspace) — Remove tauri-* deps, add new deps
Cargo.toml (cli) — Same
config/bundle.rs — Keep as-is (config API unchanged)
cli/mod.rs — Update module declarations
Reference Files (tauri source at ~/Development/Tinkering/tauri/)
crates/tauri-bundler/src/bundle/settings.rs — Settings API to mirror in BundleContext
crates/tauri-bundler/src/bundle/macos/app.rs — .app creation logic
crates/tauri-bundler/src/bundle/macos/dmg/mod.rs — DMG creation
crates/tauri-bundler/src/bundle/linux/debian.rs — .deb (pure Rust)
crates/tauri-bundler/src/bundle/linux/rpm.rs — .rpm
crates/tauri-bundler/src/bundle/linux/appimage/ — AppImage
crates/tauri-bundler/src/bundle/windows/nsis/mod.rs — NSIS
crates/tauri-bundler/src/bundle/windows/msi/mod.rs — WiX MSI
Verification
dx bundle --package-types macos produces a signed .app on macOS
dx bundle --package-types dmg produces a .dmg on macOS
dx bundle --package-types deb produces a .deb on Linux
dx bundle --package-types rpm produces an .rpm on Linux
dx bundle --package-types appimage produces an .AppImage on Linux
dx bundle --package-types nsis produces an .exe installer on Windows
dx bundle --package-types msi produces an .msi on Windows
cargo check -p dioxus-cli passes with no tauri-* imports
Existing Dioxus.toml config options still work unchanged
