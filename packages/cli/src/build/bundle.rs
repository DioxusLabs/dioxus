//! ## Web:
//! Create a folder that is somewhat similar to an app-image (exe + asset)
//! The server is dropped into the `web` folder, even if there's no `public` folder.
//! If there's no server (SPA), we still use the `web` folder, but it only contains the
//! public folder.
//! ```
//! web/
//!     server
//!     assets/
//!     public/
//!         index.html
//!         wasm/
//!            app.wasm
//!            glue.js
//!            snippets/
//!                ...
//!         assets/
//!            logo.png
//! ```
//!
//! ## Linux:
//! https://docs.appimage.org/reference/appdir.html#ref-appdir
//! current_exe.join("Assets")
//! ```
//! app.appimage/
//!     AppRun
//!     app.desktop
//!     package.json
//!     assets/
//!         logo.png
//! ```
//!
//! ## Macos
//! We simply use the macos format where binaries are in `Contents/MacOS` and assets are in `Contents/Resources`
//! We put assets in an assets dir such that it generally matches every other platform and we can
//! output `/assets/blah` from manganis.
//! ```
//! App.app/
//!     Contents/
//!         Info.plist
//!         MacOS/
//!             Frameworks/
//!         Resources/
//!             assets/
//!                 blah.icns
//!                 blah.png
//!         CodeResources
//!         _CodeSignature/
//! ```
//!
//! ## iOS
//! Not the same as mac! ios apps are a bit "flattened" in comparison. simpler format, presumably
//! since most ios apps don't ship frameworks/plugins and such.
//!
//! todo(jon): include the signing and entitlements in this format diagram.
//! ```
//! App.app/
//!     main
//!     assets/
//! ```
//!
//! ## Android:
//!
//! Currently we need to generate a `src` type structure, not a pre-packaged apk structure, since
//! we need to compile kotlin and java. This pushes us into using gradle and following a structure
//! similar to that of cargo mobile2. Eventually I'd like to slim this down (drop buildSrc) and
//! drive the kotlin build ourselves. This would let us drop gradle (yay! no plugins!) but requires
//! us to manage dependencies (like kotlinc) ourselves (yuck!).
//!
//! https://github.com/WanghongLin/miscellaneous/blob/master/tools/build-apk-manually.sh
//!
//! Unfortunately, it seems that while we can drop the `android` build plugin, we still will need
//! gradle since kotlin is basically gradle-only.
//!
//! Pre-build:
//! ```
//! app.apk/
//!     .gradle
//!     app/
//!         src/
//!             main/
//!                 assets/
//!                 jniLibs/
//!                 java/
//!                 kotlin/
//!                 res/
//!                 AndroidManifest.xml
//!             build.gradle.kts
//!             proguard-rules.pro
//!         buildSrc/
//!             build.gradle.kts
//!             src/
//!                 main/
//!                     kotlin/
//!                          BuildTask.kt
//!     build.gradle.kts
//!     gradle.properties
//!     gradlew
//!     gradlew.bat
//!     settings.gradle
//! ```
//!
//! Final build:
//! ```
//! app.apk/
//!   AndroidManifest.xml
//!   classes.dex
//!   assets/
//!       logo.png
//!   lib/
//!       armeabi-v7a/
//!           libmyapp.so
//!       arm64-v8a/
//!           libmyapp.so
//! ```
//! Notice that we *could* feasibly build this ourselves :)
//!
//! ## Windows:
//! https://superuser.com/questions/749447/creating-a-single-file-executable-from-a-directory-in-windows
//! Windows does not provide an AppImage format, so instead we're going build the same folder
//! structure as an AppImage, but when distributing, we'll create a .exe that embeds the resources
//! as an embedded .zip file. When the app runs, it will implicitly unzip its resources into the
//! Program Files folder. Any subsequent launches of the parent .exe will simply call the AppRun.exe
//! entrypoint in the associated Program Files folder.
//!
//! This is, in essence, the same as an installer, so we might eventually just support something like msi/msix
//! which functionally do the same thing but with a sleeker UI.
//!
//! This means no installers are required and we can bake an updater into the host exe.
//!
//! ## Handling asset lookups:
//! current_exe.join("assets")
//! ```
//! app.appimage/
//!     main.exe
//!     main.desktop
//!     package.json
//!     assets/
//!         logo.png
//! ```
//!
//! Since we support just a few locations, we could just search for the first that exists
//! - usr
//! - ../Resources
//! - assets
//! - Assets
//! - $cwd/assets
//!
//! ```
//! assets::root() ->
//!     mac -> ../Resources/
//!     ios -> ../Resources/
//!     android -> assets/
//!     server -> assets/
//!     liveview -> assets/
//!     web -> /assets/
//! root().join(bundled)
//! ```

use super::prerender::pre_render_static_routes;
use crate::{BuildMode, BuildRequest, Platform, WasmOptConfig};
use crate::{Result, TraceSrc};
use anyhow::{bail, Context};
use dioxus_cli_opt::{process_file_to, AssetManifest};
use itertools::Itertools;
use manganis::{AssetOptions, JsAssetOptions};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::{collections::HashSet, io::Write};
use std::{future::Future, time::Instant};
use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use std::{pin::Pin, time::SystemTime};
use std::{process::Stdio, sync::atomic::Ordering};
use std::{sync::atomic::AtomicUsize, time::Duration};
use target_lexicon::{Environment, OperatingSystem};
use tokio::process::Command;

// / The end result of a build.
// /
// / Contains the final asset manifest, the executables, and the workdir.
// /
// / Every dioxus app can have an optional server executable which will influence the final bundle.
// / This is built in parallel with the app executable during the `build` phase and the progres/status
// / of the build is aggregated.
// /
// / The server will *always* be dropped into the `web` folder since it is considered "web" in nature,
// / and will likely need to be combined with the public dir to be useful.
// /
// / We do our best to assemble read-to-go bundles here, such that the "bundle" step for each platform
// / can just use the build dir
// /
// / When we write the AppBundle to a folder, it'll contain each bundle for each platform under the app's name:
// / ```
// / dog-app/
// /   build/
// /       web/
// /         server.exe
// /         assets/
// /           some-secret-asset.txt (a server-side asset)
// /         public/
// /           index.html
// /           assets/
// /             logo.png
// /       desktop/
// /          App.app
// /          App.appimage
// /          App.exe
// /          server/
// /              server
// /              assets/
// /                some-secret-asset.txt (a server-side asset)
// /       ios/
// /          App.app
// /          App.ipa
// /       android/
// /          App.apk
// /   bundle/
// /       build.json
// /       Desktop.app
// /       Mobile_x64.ipa
// /       Mobile_arm64.ipa
// /       Mobile_rosetta.ipa
// /       web.appimage
// /       web/
// /         server.exe
// /         assets/
// /             some-secret-asset.txt
// /         public/
// /             index.html
// /             assets/
// /                 logo.png
// /                 style.css
// / ```
// /
// / When deploying, the build.json file will provide all the metadata that dx-deploy will use to
// / push the app to stores, set up infra, manage versions, etc.
// /
// / The format of each build will follow the name plus some metadata such that when distributing you
// / can easily trim off the metadata.
// /
// / The idea here is that we can run any of the programs in the same way that they're deployed.
// /
// /
// / ## Bundle structure links
// / - apple: https://developer.apple.com/documentation/bundleresources/placing_content_in_a_bundle
// / - appimage: https://docs.appimage.org/packaging-guide/manual.html#ref-manual
// /
// / ## Extra links
// / - xbuild: https://github.com/rust-mobile/xbuild/blob/master/xbuild/src/command/build.rs
// pub(crate) struct BuildArtifacts {
//     pub(crate) build: BuildRequest,
//     pub(crate) exe: PathBuf,
//     pub(crate) direct_rustc: Vec<String>,
//     pub(crate) time_start: SystemTime,
//     pub(crate) time_end: SystemTime,
//     pub(crate) assets: AssetManifest,
// }

// impl AppBundle {}
