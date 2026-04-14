//! Hotpatching: Fat and Thin Linking
//!
//! This module implements the dance we need to perform around manually linking projects using dx itself.
//! This is done by being the `RUSTC_WORKSPACE_WRAPPER` as well as `LINKER`. By intercepting both of these,
//! we can perform various optimizations like persisting rustc arguments for hotpatching.
//!
//! The flow looks like this
//! - bust fingerprint for tip of crate to ensure we always get final linker invocation
//! - run build, intercepting with wrapper, dumping rustc args to disk
//! - read dumped rustc args into a local cache for the given build under artifacts
//! - track changed crates
//! - for each changed crate, re-run the direct rustc invocation all the way to the tip
//! - relink the rlib set with the `.o` from the tip -> final binary
//!
//! source of truth is the read-out of the link args after the initial build

use super::HotpatchModuleCache;
use crate::{BuildArtifacts, BuildMode, WorkspaceRustcArgs};
use crate::{BuildContext, Error, LinkerFlavor, Result, RustcArgs, Workspace};
use crate::{BuildRequest, DX_RUSTC_WRAPPER_ENV_VAR};
use anyhow::{bail, ensure, Context};
use itertools::Itertools;
use serde::Serialize;
use sha1::Digest;
use sha2::Sha256;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ffi::OsString,
};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use subsecond_types::JumpTable;
use target_lexicon::{Architecture, OperatingSystem};
use tokio::process::Command;
use uuid::Uuid;

impl BuildRequest {
    /// We're going to create a DAG of modified crates, replay their rustc commands directly, and then
    /// manually link at the end.
    ///
    /// # Compilation
    ///
    /// We compile dirty crates by computing a dag across the workspace and then replaying the original
    /// rustc commands that generated their artifacts. For most crates, this results in an rlib being
    /// written to disk. In the case of hotpatching, the rlib is overwritten in-place since we're
    /// replaying the original rustc command. The nice thing here is that the rlibs remain stable
    /// in the linking command we've captured. The `.o` files in the linking command almost always
    /// come from the main tip crate.
    ///
    /// # Linking
    ///
    /// Run our custom linker setup to generate a patch file in the right location
    ///
    /// This should be the only case where the cargo output is a "dummy" file and requires us to
    /// manually do any linking.
    ///
    /// We also run some post processing steps here, like extracting out any new assets.
    ///
    /// Workspace support replays captured rustc invocations into the modified crate chain first,
    /// updating their on-disk outputs in place. The final patch link then combines the tip crate's
    /// fresh `.rcgu.o` files with the updated workspace rlibs from that replay.
    ///
    /// # Stub creation
    ///
    /// During this phase, we call out to `create_undefined_symbol_stub`. This function reads the
    /// rlibs and .o files that are about to be linked, identifies missing symbols, and then generates
    /// new assembly on the fly that satisfies these missing symbols. The assembly we generate outputs
    /// new functions with the corresponding symbol name that jump into known addresses of the originally
    /// loaded binary that's running and receiving patch updates.
    ///
    /// On wasm, we don't call this since WASM is much more complex and actually requires a full rewrite
    /// of the final binary. The `--allow-undefined` flag of wasm-ld lets us generate unrunnable binaries
    /// that we then fixup for load.
    ///
    /// # Linking command format
    ///
    /// When rustc links your project, it passes the args as how a linker would expect, but with
    /// a somewhat reliable ordering. These are all internal details to cargo/rustc, so we can't
    /// rely on them *too* much, but the *are* fundamental to how rust compiles your projects, and
    /// linker interfaces probably won't change drastically for another 40 years.
    ///
    /// We need to tear apart this command and only pass the args that are relevant to our thin link.
    /// Mainly, we don't want any dependency (non-workspace) rlibs to be linked. Occasionally some
    /// libraries like objc_exception export a folder with their artifacts - unsure if we actually
    /// need to include them. Generally you can err on the side that most *libraries* don't need to
    /// be linked here since dlopen satisfies those symbols anyways when the binary is loaded. In the
    /// future, if there are weird issues with a non-rust crate being linked incorrectly during hotpatch,
    /// the logic here would be a good place to check first.
    ///
    /// The format of this command roughly follows:
    /// ```
    /// clang
    ///     /dioxus/target/debug/subsecond-cli
    ///     /var/folders/zs/gvrfkj8x33d39cvw2p06yc700000gn/T/rustcAqQ4p2/symbols.o
    ///     /dioxus/target/subsecond-dev/deps/subsecond_harness-acfb69cb29ffb8fa.05stnb4bovskp7a00wyyf7l9s.rcgu.o
    ///     /dioxus/target/subsecond-dev/deps/subsecond_harness-acfb69cb29ffb8fa.08rgcutgrtj2mxoogjg3ufs0g.rcgu.o
    ///     /dioxus/target/subsecond-dev/deps/subsecond_harness-acfb69cb29ffb8fa.0941bd8fa2bydcv9hfmgzzne9.rcgu.o
    ///     /dioxus/target/subsecond-dev/deps/libbincode-c215feeb7886f81b.rlib
    ///     /dioxus/target/subsecond-dev/deps/libanyhow-e69ac15c094daba6.rlib
    ///     /dioxus/target/subsecond-dev/deps/libratatui-c3364579b86a1dfc.rlib
    ///     /.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/lib/libstd-019f0f6ae6e6562b.rlib
    ///     /.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/lib/libpanic_unwind-7387d38173a2eb37.rlib
    ///     /.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/lib/libobject-2b03cf6ece171d21.rlib
    ///     -framework AppKit
    ///     -lc
    ///     -framework Foundation
    ///     -framework Carbon
    ///     -lSystem
    ///     -framework CoreFoundation
    ///     -lobjc
    ///     -liconv
    ///     -lm
    ///     -arch arm64
    ///     -mmacosx-version-min=11.0.0
    ///     -L /dioxus/target/subsecond-dev/build/objc_exception-dc226cad0480ea65/out
    ///     -o /dioxus/target/subsecond-dev/deps/subsecond_harness-acfb69cb29ffb8fa
    ///     -nodefaultlibs
    ///     -Wl,-all_load
    /// ```
    ///
    /// Many args are passed twice, too, which can be confusing, but generally don't have any real
    /// effect. Note that on macos/ios, there's a special macho header that needs to be set, otherwise
    /// dyld will complain.
    ///
    /// Also, some flags in darwin land might become deprecated, need to be super conservative:
    /// - <https://developer.apple.com/forums/thread/773907>
    ///
    /// We need to be careful about which linker we're interpreting too. Some are old, some are new,
    /// some are experimental, and each has their own syntax ie `-C, /C, --C, C=` which need to be handlded.
    pub async fn compile_workspace_hotpatch(&self, ctx: &BuildContext) -> Result<BuildArtifacts> {
        let BuildMode::Thin {
            aslr_reference,
            workspace_rustc_args,
            modified_crates,
            cache,
            ..
        } = &ctx.mode
        else {
            bail!("Not thin mode!")
        };

        tracing::debug!("Changed crates dag using {modified_crates:?}");

        // Replay the rustcs for all modified workspace crates. This is not the final tip binary.
        // Note that the final tip might include itself as a lib (lib.rs + main.rs) which gets covered here.
        ctx.profile_phase("Workspace hotpatch replay");
        let replayed_crates = self.workspace_hotpatch_replay_order(modified_crates)?;
        tracing::debug!("replaying crates: {replayed_crates:?}");
        for crate_name in &replayed_crates {
            let rustc_args = self
                .workspace_hotpatch_replay_args(workspace_rustc_args, crate_name)
                .with_context(|| format!("Missing rustc args for replay: '{crate_name}'"))?;
            self.compile_dep_crate(crate_name, rustc_args)
                .await
                .with_context(|| format!("Failed to replay workspace crate '{crate_name}'"))?;
        }

        // Recompile just the tip crate now
        let mut artifacts = self.cargo_build(ctx).await?;

        ctx.status_writing_patch();
        ctx.profile_phase("Patch: Cache Tip Objects");

        // Cache tip crate objects from the FRESH linker args (from the just-completed
        // thin build, not the stale ones from ctx.mode's fat build).
        let link_args = &artifacts.workspace_rustc.link_args;
        let tip_bin_key = format!("{}.bin", self.tip_crate_name());
        let args = artifacts
            .workspace_rustc
            .rustc_args
            .get(&tip_bin_key)
            .cloned()
            .with_context(|| {
                format!(
                    "Missing rustc args for tip bin target '{tip_bin_key}' \
                     (available keys: {:?})",
                    artifacts
                        .workspace_rustc
                        .rustc_args
                        .keys()
                        .collect::<Vec<_>>()
                )
            })?;

        let mut dylibs = vec![];

        // Tip objects from link_args are temps — safe to delete after linking.
        let temp_objects: Vec<PathBuf> = artifacts
            .workspace_rustc
            .link_args
            .iter()
            .filter(|arg| arg.ends_with(".rcgu.o"))
            .sorted()
            .map(PathBuf::from)
            .collect();

        let workspace_rlibs =
            self.workspace_hotpatch_link_rlibs(&artifacts.workspace_rustc, &replayed_crates)?;

        // Merge both sets for the linker. Merge order
        let mut object_files: Vec<PathBuf> = temp_objects.clone();
        object_files.extend(workspace_rlibs.iter().cloned());

        // On non-wasm platforms, we generate a special shim object file which converts symbols from
        // fat binary into direct addresses from the running process.
        //
        // Our wasm approach is quite specific to wasm. We don't need to resolve any missing symbols
        // there since wasm is relocatable, but there is considerable pre and post processing work to
        // satisfy undefined symbols that we do by munging the binary directly.
        //
        // todo: can we adjust our wasm approach to also use a similar system?
        // todo: don't require the aslr reference and just patch the got when loading.
        //
        // Requiring the ASLR offset here is necessary but unfortunately might be flakey in practice.
        // Android apps can take a long time to open, and a hot patch might've been issued in the interim,
        // making this hotpatch a failure.
        if !self.is_wasm_or_wasi() {
            let stub_bytes = crate::build::create_undefined_symbol_stub(
                cache,
                &object_files,
                &self.triple,
                *aslr_reference,
            )
            .expect("failed to resolve patch symbols");

            // Currently we're dropping stub.o in the exe dir, but should probably just move to a tempfile?
            let patch_file = self.main_exe().with_file_name("stub.o");
            std::fs::write(&patch_file, stub_bytes)?;
            object_files.push(patch_file);

            // Add the dylibs/sos to the linker args
            // Make sure to use the one in the bundle, not the ones in the target dir or system.
            for arg in &artifacts.workspace_rustc.link_args {
                if arg.ends_with(".dylib") || arg.ends_with(".so") {
                    let path = PathBuf::from(arg);
                    dylibs.push(self.frameworks_folder().join(path.file_name().unwrap()));
                }
            }
        }

        // And now we can run the linker with our new args
        let linker = self.select_linker()?;
        let out_exe = self.patch_exe(artifacts.time_start);
        let out_arg = match self.triple.operating_system {
            OperatingSystem::Windows => vec![format!("/OUT:{}", out_exe.display())],
            _ => vec!["-o".to_string(), out_exe.display().to_string()],
        };

        tracing::trace!("Linking with {:?} using args: {:#?}", linker, object_files);
        tracing::trace!("Workspace hotpatch rlibs: {:#?}", workspace_rlibs);

        let mut out_args: Vec<OsString> = vec![];
        out_args.extend(object_files.iter().map(Into::into));
        out_args.extend(dylibs.iter().map(Into::into));
        out_args.extend(
            self.thin_link_args(&artifacts.workspace_rustc.link_args)?
                .iter()
                .map(Into::into),
        );
        out_args.extend(out_arg.iter().map(Into::into));

        if cfg!(windows) {
            let cmd_contents: String = out_args
                .iter()
                .map(|s| format!("\"{}\"", s.to_string_lossy()))
                .join(" ");
            std::fs::write(self.windows_command_file(), cmd_contents)
                .context("Failed to write linker command file")?;
            out_args = vec![format!("@{}", self.windows_command_file().display()).into()];
        }

        // Add more search paths for the linker
        let mut command_envs = args.envs.clone();

        // On linux, we need to set a more complete PATH for the linker to find its libraries
        if cfg!(target_os = "linux") {
            command_envs.push(("PATH".to_string(), std::env::var("PATH").unwrap()));
        }

        // Run the linker directly!
        //
        // We dump its output directly into the patch exe location which is different than how rustc
        // does it since it uses llvm-objcopy into the `target/debug/` folder.
        ctx.profile_phase("Patch: Link");
        let res = Command::new(linker)
            .args(out_args)
            .env_clear()
            .envs(command_envs)
            .output()
            .await?;

        if !res.stderr.is_empty() {
            let errs = String::from_utf8_lossy(&res.stderr);
            if !self.patch_exe(artifacts.time_start).exists() || !res.status.success() {
                tracing::error!(
                    telemetry = %serde_json::json!({ "event": "hotpatch_linker_failed" }),
                    "Failed to generate patch: {}",
                    errs.trim()
                );
            } else {
                tracing::trace!("Linker output during thin linking: {}", errs.trim());
            }
        }

        // For some really weird reason that I think is because of dlopen caching, future loads of the
        // jump library will fail if we don't remove the original fat file. I think this could be
        // because of library versioning and namespaces, but really unsure.
        //
        // The errors if you forget to do this are *extremely* cryptic - missing symbols that never existed.
        //
        // Fortunately, this binary exists in two places - the deps dir and the target out dir. We
        // can just remove the one in the deps dir and the problem goes away.
        if let Some(idx) = link_args.iter().position(|arg| *arg == "-o") {
            _ = std::fs::remove_file(PathBuf::from(link_args[idx + 1].as_str()));
        }

        // Clean up temp object files (tip incremental objects + stub.o).
        // Cached dep objects in object_cache/ are NOT deleted — they persist across patches.
        for file in &temp_objects {
            _ = std::fs::remove_file(file);
        }

        // Now extract linker metadata from the fat binary (assets, plugin data)
        artifacts.assets = self
            .collect_assets_and_metadata(&self.patch_exe(artifacts.time_start), ctx)
            .await?;

        // If this is a web build, reset the index.html file in case it was modified by SSG
        self.write_index_html(&artifacts.assets)
            .context("Failed to write index.html")?;

        Ok(artifacts)
    }

    /// Take the original args passed to the "fat" build and then create the "thin" variant.
    ///
    /// This is basically just stripping away the rlibs and other libraries that will be satisfied
    /// by our stub step.
    fn thin_link_args(&self, original_args: &[String]) -> Result<Vec<String>> {
        let mut out_args = vec![];

        match self.linker_flavor() {
            // wasm32-unknown-unknown -> use wasm-ld (gnu-lld)
            //
            // We need to import a few things - namely the memory and ifunc table.
            //
            // We can safely export everything, I believe, though that led to issues with the "fat"
            // binaries that also might lead to issues here too. wasm-bindgen chokes on some symbols
            // and the resulting JS has issues.
            //
            // We turn on both --pie and --experimental-pic but I think we only need --pie.
            //
            // We don't use *any* of the original linker args since they do lots of custom exports
            // and other things that we don't need.
            //
            // The trickiest one here is -Crelocation-model=pic, which forces data symbols
            // into a GOT, making it possible to import them from the main module.
            //
            // I think we can make relocation-model=pic work for non-wasm platforms, enabling
            // fully relocatable modules with no host coordination in lieu of sending out
            // the aslr slide at runtime.
            LinkerFlavor::WasmLld => {
                out_args.extend([
                    "--fatal-warnings".to_string(),
                    "--verbose".to_string(),
                    "--import-memory".to_string(),
                    "--import-table".to_string(),
                    "--growable-table".to_string(),
                    "--export".to_string(),
                    "main".to_string(),
                    "--allow-undefined".to_string(),
                    "--no-demangle".to_string(),
                    "--no-entry".to_string(),
                    "--pie".to_string(),
                    "--experimental-pic".to_string(),
                ]);

                // retain exports so post-processing has hooks to work with
                for (idx, arg) in original_args.iter().enumerate() {
                    if *arg == "--export" {
                        out_args.push(arg.to_string());
                        out_args.push(original_args[idx + 1].to_string());
                    }
                }
            }

            // This uses "cc" and these args need to be ld compatible
            //
            // Most importantly, we want to pass `-dylib` to both CC and the linker to indicate that
            // we want to generate the shared library instead of an executable.
            LinkerFlavor::Darwin => {
                out_args.extend(["-Wl,-dylib".to_string()]);

                // Preserve the original args. We only preserve:
                // -framework
                // -arch
                // -lxyz
                // There might be more, but some flags might break our setup.
                for (idx, arg) in original_args.iter().enumerate() {
                    if *arg == "-framework"
                        || *arg == "-arch"
                        || *arg == "-L"
                        || *arg == "-target"
                        || (*arg == "-isysroot"
                            && matches!(
                                self.triple.operating_system,
                                target_lexicon::OperatingSystem::IOS(_)
                            ))
                    {
                        out_args.push(arg.to_string());
                        out_args.push(original_args[idx + 1].to_string());
                    }

                    if arg.starts_with("-l")
                        || arg.starts_with("-m")
                        || arg.starts_with("-nodefaultlibs")
                    {
                        out_args.push(arg.to_string());
                    }
                }
            }

            // android/linux need to be compatible with lld
            //
            // android currently drags along its own libraries and other zany flags
            LinkerFlavor::Gnu => {
                out_args.extend([
                    "-shared".to_string(),
                    "-Wl,--eh-frame-hdr".to_string(),
                    "-Wl,-z,noexecstack".to_string(),
                    "-Wl,-z,relro,-z,now".to_string(),
                    "-nodefaultlibs".to_string(),
                    "-Wl,-Bdynamic".to_string(),
                ]);

                // Preserve the original args. We only preserve:
                // -L <path>
                // -lxyz
                // -m (arch/emulation)
                // -B<path>  (gcc program search path — Rust 1.86+ injects -B/gcc-ld + -fuse-ld=lld
                //            so that cc picks up the bundled lld; we must forward it for the patch
                //            linker invocation too, otherwise cc falls back to the system `ld`)
                // -fuse-ld  (linker selection)
                // There might be more, but some flags might break our setup.
                for (idx, arg) in original_args.iter().enumerate() {
                    if *arg == "-L" {
                        out_args.push(arg.to_string());
                        out_args.push(original_args[idx + 1].to_string());
                    }

                    if arg.starts_with("-l")
                        || arg.starts_with("-m")
                        || arg.starts_with("-Wl,--target=")
                        || arg.starts_with("-Wl,-fuse-ld")
                        || arg.starts_with("-fuse-ld")
                        || arg.starts_with("-B")
                        || arg.contains("-ld-path")
                    {
                        out_args.push(arg.to_string());
                    }
                }
            }

            LinkerFlavor::Msvc => {
                out_args.extend([
                    "shlwapi.lib".to_string(),
                    "kernel32.lib".to_string(),
                    "advapi32.lib".to_string(),
                    "ntdll.lib".to_string(),
                    "userenv.lib".to_string(),
                    "ws2_32.lib".to_string(),
                    "dbghelp.lib".to_string(),
                    "/defaultlib:msvcrt".to_string(),
                    "/DLL".to_string(),
                    "/DEBUG".to_string(),
                    "/PDBALTPATH:%_PDB%".to_string(),
                    "/EXPORT:main".to_string(),
                    "/HIGHENTROPYVA:NO".to_string(),
                ]);
            }

            LinkerFlavor::Unsupported => {
                bail!("Unsupported platform for thin linking")
            }
        }

        let extract_value = |arg: &str| -> Option<String> {
            original_args
                .iter()
                .position(|a| *a == arg)
                .map(|i| original_args[i + 1].to_string())
        };

        if let Some(vale) = extract_value("-target") {
            out_args.push("-target".to_string());
            out_args.push(vale);
        }

        if let Some(vale) = extract_value("-isysroot") {
            if matches!(
                self.triple.operating_system,
                target_lexicon::OperatingSystem::IOS(_)
            ) {
                out_args.push("-isysroot".to_string());
                out_args.push(vale);
            }
        }

        Ok(out_args)
    }

    /// Compile a workspace crate directly with `rustc` using its captured args.
    ///
    /// This produces updated outputs at the same paths cargo originally wrote to.
    /// Used during thin builds to replay the modified workspace chain before the tip crate.
    async fn compile_dep_crate(&self, crate_name: &str, rustc_args: &RustcArgs) -> Result<()> {
        let mut cmd = Command::new("rustc");
        cmd.current_dir(self.workspace_dir());
        cmd.env_clear();

        // Skip args[0] which is the rustc binary path captured by the wrapper.
        // We must also strip the dx linker override so replayed crates produce real outputs
        // instead of re-entering our no-link interception path.
        let mut replay_args = Vec::with_capacity(rustc_args.args.len().saturating_sub(1));
        let mut idx = 1;
        while idx < rustc_args.args.len() {
            let arg = &rustc_args.args[idx];

            if arg.starts_with("-Clinker=") {
                idx += 1;
                continue;
            }

            if arg == "-C"
                && rustc_args
                    .args
                    .get(idx + 1)
                    .is_some_and(|next| next.starts_with("linker="))
            {
                idx += 2;
                continue;
            }

            replay_args.push(arg.clone());
            idx += 1;
        }

        cmd.args(&replay_args);

        // Restore the captured environment, filtering out wrapper env vars and
        // stale cargo jobserver vars to prevent recursive invocation and warnings.
        let filtered_env_keys = [
            "RUSTC_WORKSPACE_WRAPPER",
            "RUSTC_WRAPPER",
            DX_RUSTC_WRAPPER_ENV_VAR,
            "CARGO_MAKEFLAGS",
            "MAKEFLAGS",
        ];
        cmd.envs(
            rustc_args
                .envs
                .iter()
                .filter(|(k, _)| {
                    !filtered_env_keys.contains(&k.as_str()) && !k.starts_with("DX_LINK")
                })
                .cloned(),
        );

        // Wasm hotpatches are linked as relocatable PIC modules, so replayed workspace crate
        // compilations need to emit PIC-compatible objects too.
        if self.is_wasm_or_wasi() {
            cmd.arg("-Crelocation-model=pic");
        }

        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to compile workspace dep crate '{crate_name}':\n{stderr}");
        }

        Ok(())
    }

    fn workspace_hotpatch_replay_args<'a>(
        &self,
        workspace_rustc_args: &'a WorkspaceRustcArgs,
        crate_name: &str,
    ) -> Option<&'a RustcArgs> {
        let lib_key = format!("{crate_name}.lib");
        // if crate_name == self.tip_crate_name() {
        //     return workspace_rustc_args
        //         .rustc_args
        //         .get(&format!("{crate_name}.bin"));
        // }

        workspace_rustc_args.rustc_args.get(&lib_key).or_else(|| {
            workspace_rustc_args
                .rustc_args
                .get(&format!("{crate_name}.bin"))
        })
    }

    /// Topological sort of modified workspace crates for rustc replay.
    ///
    /// The caller (builder) already guarantees that every crate in `modified_crates`
    /// transitively reaches the tip. This function excludes the tip crate itself — it
    /// gets compiled separately via `cargo_build` after the replay. The remaining lib
    /// crates are ordered so dependencies compile before dependents (Kahn's algorithm).
    /// Ties are broken lexicographically for determinism.
    fn workspace_hotpatch_replay_order(
        &self,
        modified_crates: &HashSet<String>,
    ) -> Result<Vec<String>> {
        // Exclude the tip crate — it's compiled separately via cargo_build after replay.
        let tip = self.tip_crate_name();
        let crates: HashSet<&String> = modified_crates
            .iter()
            .filter(|name| **name != tip)
            .collect();

        // Build the subgraph: edge A→B means "A must compile before B".
        let mut indegree: HashMap<&String, usize> = crates.iter().map(|name| (*name, 0)).collect();
        let mut edges: HashMap<&String, Vec<&String>> = HashMap::new();

        for crate_name in &crates {
            for dependent in self.workspace_dependents_of(crate_name) {
                if let Some(dep) = crates.get(&dependent) {
                    *indegree.entry(dep).or_default() += 1;
                    edges.entry(crate_name).or_default().push(dep);
                }
            }
        }

        // Kahn's algorithm. BTreeSet gives deterministic (lexicographic) tie-breaking.
        let mut ready: BTreeSet<&String> = indegree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| *name)
            .collect();
        let mut ordered = Vec::with_capacity(crates.len());
        while let Some(name) = ready.pop_first() {
            ordered.push(name.clone());
            for dep in edges.get(name).into_iter().flatten() {
                let deg = indegree.get_mut(dep).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    ready.insert(dep);
                }
            }
        }

        ensure!(
            ordered.len() == crates.len(),
            "Cycle in workspace dependency graph — cannot determine replay order"
        );

        Ok(ordered)
    }

    /// Collect the rlib paths for every replayed workspace crate, ordered for the linker.
    ///
    /// Each crate in `replayed_crates` is resolved to its on-disk `.rlib` using the captured
    /// rustc args from the fat build (specifically `--out-dir` and `-C extra-filename`).
    /// Every crate must resolve — a missing rlib would produce a corrupted patch binary.
    ///
    /// The returned paths preserve the original link order from the fat build's captured
    /// linker arguments. Any rlibs not found in that order are appended at the end.
    fn workspace_hotpatch_link_rlibs(
        &self,
        args: &WorkspaceRustcArgs,
        replayed_crates: &[String],
    ) -> Result<Vec<PathBuf>> {
        // Resolve every replayed crate to its rlib path. Every crate must resolve —
        // a missing rlib means we'd link a corrupted binary.
        let mut wanted = HashSet::new();
        for crate_name in replayed_crates {
            let rustc_args = args
                .rustc_args
                .get(&format!("{crate_name}.lib"))
                .with_context(|| {
                    format!(
                        "Missing captured rustc args for workspace crate '{crate_name}.lib' \
                         (available: {:?})",
                        args.rustc_args.keys().collect::<Vec<_>>()
                    )
                })?;

            let rlib = self
                .find_rlib_for_crate(crate_name, rustc_args)
                .with_context(|| {
                    format!("Could not find rlib for workspace crate '{crate_name}'")
                })?;

            wanted.insert(rlib);
        }

        // Preserve the link order from the original fat build for any rlibs that appear
        // in the captured link args.
        let mut ordered = Vec::new();
        let mut seen = HashSet::new();

        for arg in &args.link_args {
            if !arg.ends_with(".rlib") {
                continue;
            }

            let path = PathBuf::from(arg);
            if wanted.contains(&path) && seen.insert(path.clone()) {
                ordered.push(path);
            }
        }

        // Any rlibs not in the captured link order get appended at the end.
        let mut remaining: Vec<_> = wanted.into_iter().filter(|p| !seen.contains(p)).collect();
        remaining.sort();
        ordered.extend(remaining);

        Ok(ordered)
    }

    /// Patches are stored in the same directory as the main executable, but with a name based on the
    /// time the patch started compiling.
    ///
    /// - lib{name}-patch-{time}.(so/dll/dylib) (next to the main exe)
    ///
    /// Note that weirdly enough, the name of dylibs can actually matter. In some environments, libs
    /// can override each other with symbol interposition.
    ///
    /// Also, on Android - and some Linux, we *need* to start the lib name with `lib` for the dynamic
    /// loader to consider it a shared library.
    ///
    /// todo: the time format might actually be problematic if two platforms share the same build folder.
    pub(crate) fn patch_exe(&self, time_start: SystemTime) -> PathBuf {
        let path = self.main_exe().with_file_name(format!(
            "lib{}-patch-{}",
            self.executable_name(),
            time_start
                .duration_since(UNIX_EPOCH)
                .map(|f| f.as_millis())
                .unwrap_or(0),
        ));

        let extension = match self.linker_flavor() {
            LinkerFlavor::Darwin => "dylib",
            LinkerFlavor::Gnu => "so",
            LinkerFlavor::WasmLld => "wasm",
            LinkerFlavor::Msvc => "dll",
            LinkerFlavor::Unsupported => "",
        };

        path.with_extension(extension)
    }

    /// When we link together the fat binary, we need to make sure every `.o` file in *every* rlib
    /// is taken into account. This is the same work that the rust compiler does when assembling
    /// staticlibs.
    ///
    /// <https://github.com/rust-lang/rust/blob/191df20fcad9331d3a948aa8e8556775ec3fe69d/compiler/rustc_codegen_ssa/src/back/link.rs#L448>
    ///
    /// Since we're going to be passing these to the linker, we need to make sure and not provide any
    /// weird files (like the rmeta) file that rustc generates.
    ///
    /// We discovered the need for this after running into issues with wasm-ld not being able to
    /// handle the rmeta file.
    ///
    /// <https://github.com/llvm/llvm-project/issues/55786>
    ///
    /// Also, crates might not drag in all their dependent code. The monorphizer won't lift trait-based generics:
    ///
    /// <https://github.com/rust-lang/rust/blob/191df20fcad9331d3a948aa8e8556775ec3fe69d/compiler/rustc_monomorphize/src/collector.rs>
    ///
    /// When Rust normally handles this, it uses the +whole-archive directive which adjusts how the rlib
    /// is written to disk.
    ///
    /// Since creating this object file can be a lot of work, we cache it in the target dir by hashing
    /// the names of the rlibs in the command and storing it in the target dir. That way, when we run
    /// this command again, we can just used the cached object file.
    ///
    /// In theory, we only need to do this for every crate accessible by the current crate, but that's
    /// hard acquire without knowing the exported symbols from each crate.
    ///
    /// todo: I think we can traverse our immediate dependencies and inspect their symbols, unless they `pub use` a crate
    /// todo: we should try and make this faster with memmapping
    pub(crate) async fn run_fat_link(
        &self,
        ctx: &BuildContext,
        exe: &Path,
        set: &WorkspaceRustcArgs,
    ) -> Result<()> {
        // Get the tip crate rustc argsa
        let rustc_args = set
            .rustc_args
            .get(&format!("{}.bin", self.tip_crate_name()))
            .context("Missing rustc capture")?;

        ensure!(
            !set.link_args.is_empty(),
            "Missing linker args for fat link of '{}'. The tip crate likely did not run through linker interception for this build.",
            self.tip_crate_name()
        );

        let link_start = SystemTime::now();
        ctx.status_starting_fat_link();

        // Filter out the rlib files from the arguments
        let rlibs = set
            .link_args
            .iter()
            .filter(|arg| arg.ends_with(".rlib"))
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        // Acquire a hash from the rlib names, sizes, modified times, and dx's git commit hash
        // This ensures that any changes in dx or the rlibs will cause a new hash to be generated
        // The hash relies on both dx and rustc hashes, so it should be thoroughly unique. Keep it
        // short to avoid long file names.
        let hash_id = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            rlibs
                .iter()
                .map(|p| {
                    format!(
                        "{}-{}-{}-{}",
                        p.file_name().unwrap().to_string_lossy(),
                        p.metadata().map(|m| m.len()).unwrap_or_default(),
                        p.metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .and_then(|f| f.duration_since(UNIX_EPOCH).map(|f| f.as_secs()).ok())
                            .unwrap_or_default(),
                        crate::dx_build_info::GIT_COMMIT_HASH.unwrap_or_default()
                    )
                })
                .collect::<String>()
                .as_bytes(),
        )
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();

        // Check if we already have a cached object file
        let out_ar_path = exe.with_file_name(format!("libdeps-{hash_id}.a",));
        let out_rlibs_list = exe.with_file_name(format!("rlibs-{hash_id}.txt"));
        let mut archive_has_contents = out_ar_path.exists();

        // Use the rlibs list if it exists
        let mut compiler_rlibs = std::fs::read_to_string(&out_rlibs_list)
            .ok()
            .map(|s| s.lines().map(PathBuf::from).collect::<Vec<_>>())
            .unwrap_or_default();

        // Create it by dumping all the rlibs into it
        // This will include the std rlibs too, which can severely bloat the size of the archive
        //
        // The nature of this process involves making extremely fat archives, so we should try and
        // speed up the future linking process by caching the archive.
        //
        // Since we're using the git hash for the CLI entropy, debug builds should always regenerate
        // the archive since their hash might not change, but the logic might.
        if !archive_has_contents || cfg!(debug_assertions) {
            compiler_rlibs.clear();

            let mut bytes = vec![];
            let mut out_ar = ar::Builder::new(&mut bytes);
            for rlib in &rlibs {
                // Skip compiler rlibs since they're missing bitcode
                //
                // https://github.com/rust-lang/rust/issues/94232#issuecomment-1048342201
                //
                // if the rlib is not in the target directory, we skip it.
                if !rlib.starts_with(self.workspace_dir()) {
                    compiler_rlibs.push(rlib.clone());
                    tracing::trace!("Skipping rlib: {:?}", rlib);
                    continue;
                }

                tracing::trace!("Adding rlib to staticlib: {:?}", rlib);

                let rlib_contents = std::fs::read(rlib)?;
                let mut reader = ar::Archive::new(std::io::Cursor::new(rlib_contents));
                let mut keep_linker_rlib = false;
                while let Some(Ok(object_file)) = reader.next_entry() {
                    let name = std::str::from_utf8(object_file.header().identifier()).unwrap();
                    if name.ends_with(".rmeta") {
                        continue;
                    }

                    if object_file.header().size() == 0 {
                        continue;
                    }

                    // rlibs might contain dlls/sos/lib files which we don't want to include
                    //
                    // This catches .dylib, .so, .dll, .lib, .o, etc files that are not compatible with
                    // our "fat archive" linking process.
                    //
                    // We only trust `.rcgu.o` files to make it into the --all_load archive.
                    // This is a temporary stopgap to prevent issues with libraries that generate
                    // object files that are not compatible with --all_load.
                    // see https://github.com/DioxusLabs/dioxus/issues/4237
                    if !(name.ends_with(".rcgu.o") || name.ends_with(".obj")) {
                        keep_linker_rlib = true;
                        continue;
                    }

                    archive_has_contents = true;
                    out_ar
                        .append(&object_file.header().clone(), object_file)
                        .context("Failed to add object file to archive")?;
                }

                // Some rlibs contain weird artifacts that we don't want to include in the fat archive.
                // However, we still want them around in the linker in case the regular linker can handle them.
                if keep_linker_rlib {
                    compiler_rlibs.push(rlib.clone());
                }
            }

            let bytes = out_ar.into_inner().context("Failed to finalize archive")?;
            std::fs::write(&out_ar_path, bytes).context("Failed to write archive")?;
            tracing::debug!("Wrote fat archive to {:?}", out_ar_path);

            // Run the ranlib command to index the archive. This slows down this process a bit,
            // but is necessary for some linkers to work properly.
            // We ignore its error in case it doesn't recognize the architecture
            if self.linker_flavor() == LinkerFlavor::Darwin {
                if let Some(ranlib) = Workspace::select_ranlib() {
                    _ = Command::new(ranlib).arg(&out_ar_path).output().await;
                }
            }
        }

        compiler_rlibs.dedup();

        // We're going to replace the first rlib in the args with our fat archive
        // And then remove the rest of the rlibs
        //
        // We also need to insert the -force_load flag to force the linker to load the archive
        let mut args: Vec<_> = set.link_args.clone();
        if let Some(last_object) = args.iter().rposition(|arg| arg.ends_with(".o")) {
            if archive_has_contents {
                match self.linker_flavor() {
                    LinkerFlavor::WasmLld => {
                        args.insert(last_object, "--whole-archive".to_string());
                        args.insert(last_object + 1, out_ar_path.display().to_string());
                        args.insert(last_object + 2, "--no-whole-archive".to_string());
                        args.retain(|arg| !arg.ends_with(".rlib"));
                        for rlib in compiler_rlibs.iter().rev() {
                            args.insert(last_object + 3, rlib.display().to_string());
                        }
                    }
                    LinkerFlavor::Gnu => {
                        args.insert(last_object, "-Wl,--whole-archive".to_string());
                        args.insert(last_object + 1, out_ar_path.display().to_string());
                        args.insert(last_object + 2, "-Wl,--no-whole-archive".to_string());
                        args.retain(|arg| !arg.ends_with(".rlib"));
                        for rlib in compiler_rlibs.iter().rev() {
                            args.insert(last_object + 3, rlib.display().to_string());
                        }
                    }
                    LinkerFlavor::Darwin => {
                        args.insert(last_object, "-Wl,-force_load".to_string());
                        args.insert(last_object + 1, out_ar_path.display().to_string());
                        args.retain(|arg| !arg.ends_with(".rlib"));
                        for rlib in compiler_rlibs.iter().rev() {
                            args.insert(last_object + 2, rlib.display().to_string());
                        }
                    }
                    LinkerFlavor::Msvc => {
                        args.insert(
                            last_object,
                            format!("/WHOLEARCHIVE:{}", out_ar_path.display()),
                        );
                        args.retain(|arg| !arg.ends_with(".rlib"));
                        for rlib in compiler_rlibs.iter().rev() {
                            args.insert(last_object + 1, rlib.display().to_string());
                        }
                    }
                    LinkerFlavor::Unsupported => {
                        tracing::error!("Unsupported platform for fat linking: {}", self.triple);
                    }
                };
            }
        }

        // Add custom args to the linkers
        match self.linker_flavor() {
            LinkerFlavor::Gnu => {
                // Export `main` so subsecond can use it for a reference point
                args.push("-Wl,--export-dynamic-symbol,main".to_string());
            }
            LinkerFlavor::Darwin => {
                args.push("-Wl,-exported_symbol,_main".to_string());
            }
            LinkerFlavor::Msvc => {
                // Prevent alsr from overflowing 32 bits
                args.push("/HIGHENTROPYVA:NO".to_string());

                // Export `main` so subsecond can use it for a reference point
                args.push("/EXPORT:main".to_string());
            }
            LinkerFlavor::WasmLld | LinkerFlavor::Unsupported => {}
        }

        // We also need to remove the `-o` flag since we want the linker output to end up in the
        // rust exe location, not in the deps dir as it normally would.
        if let Some(idx) = args
            .iter()
            .position(|arg| *arg == "-o" || *arg == "--output")
        {
            args.remove(idx + 1);
            args.remove(idx);
        }

        // same but windows support
        if let Some(idx) = args.iter().position(|arg| arg.starts_with("/OUT")) {
            args.remove(idx);
        }

        // We want to go through wasm-ld directly, so we need to remove the -flavor flag
        if let Some(flavor_idx) = args.iter().position(|arg| *arg == "-flavor") {
            args.remove(flavor_idx + 1);
            args.remove(flavor_idx);
        }

        // Note: Swift sources are now compiled as dynamic frameworks during the main build flow.
        // Dynamic frameworks are loaded at runtime, not linked statically, so we don't add
        // them to the linker args here. The framework will be installed to the Frameworks
        // folder by compile_swift_sources() in the main bundle creation phase.
        if matches!(
            self.triple.operating_system,
            OperatingSystem::IOS(_) | OperatingSystem::MacOSX { .. } | OperatingSystem::Darwin(_)
        ) {
            let workspace_dir = self.workspace_dir();
            let swift_sources =
                super::apple::extract_swift_metadata_from_link_args(&set.link_args, &workspace_dir);

            if !swift_sources.is_empty() {
                tracing::debug!(
                    "Found {} Swift plugin source(s) - will be compiled as dynamic framework during bundle creation",
                    swift_sources.len()
                );
            }
        }

        // Set the output file
        match self.triple.operating_system {
            OperatingSystem::Windows => args.push(format!("/OUT:{}", exe.display())),
            _ => args.extend(["-o".to_string(), exe.display().to_string()]),
        }

        // And now we can run the linker with our new args
        let linker = self.select_linker()?;

        tracing::trace!("Fat linking with args: {:?} {:#?}", linker, args);
        tracing::trace!("Fat linking with env:");
        for e in rustc_args.envs.iter() {
            tracing::trace!("  {}={}", e.0, e.1);
        }

        // Handle windows command files
        let mut out_args = args.clone();
        if cfg!(windows) {
            let cmd_contents: String = out_args.iter().map(|f| format!("\"{f}\"")).join(" ");
            std::fs::write(self.windows_command_file(), cmd_contents)
                .context("Failed to write linker command file")?;
            out_args = vec![format!("@{}", self.windows_command_file().display())];
        }

        // Add more search paths for the linker
        let mut command_envs = rustc_args.envs.clone();

        // On linux, we need to set a more complete PATH for the linker to find its libraries
        if cfg!(target_os = "linux") {
            command_envs.push(("PATH".to_string(), std::env::var("PATH").unwrap()));
        }

        // Run the linker directly!
        let res = Command::new(linker)
            .args(out_args)
            .env_clear()
            .envs(command_envs)
            .output()
            .await?;

        if !res.status.success() {
            let stderr = String::from_utf8_lossy(&res.stderr);
            let stdout = String::from_utf8_lossy(&res.stdout);
            let combined = match (stdout.trim().is_empty(), stderr.trim().is_empty()) {
                (false, false) => format!("{}\n{}", stdout.trim(), stderr.trim()),
                (false, true) => stdout.trim().to_string(),
                (true, false) => stderr.trim().to_string(),
                (true, true) => format!("linker exited with status {}", res.status),
            };

            tracing::error!(
                telemetry = %serde_json::json!({ "event": "hotpatch_fat_binary_generation_failed" }),
                "Failed to generate fat binary: {}",
                combined
            );
            bail!("Failed to generate fat binary: {combined}");
        }

        if !res.stderr.is_empty() {
            let errs = String::from_utf8_lossy(&res.stderr);
            tracing::trace!("Warnings during fat linking: {}", errs.trim());
        }

        if !res.stdout.is_empty() {
            let out = String::from_utf8_lossy(&res.stdout);
            tracing::trace!("Output from fat linking: {}", out.trim());
        }

        // Clean up the temps manually
        for f in args.iter().filter(|arg| arg.ends_with(".rcgu.o")) {
            _ = std::fs::remove_file(f);
        }

        // Cache the rlibs list
        _ = std::fs::write(
            &out_rlibs_list,
            compiler_rlibs
                .into_iter()
                .map(|s| s.display().to_string())
                .join("\n"),
        );

        tracing::debug!(
            "Fat linking completed in {}us",
            SystemTime::now()
                .duration_since(link_start)
                .unwrap()
                .as_micros()
        );

        Ok(())
    }

    pub(crate) fn create_jump_table(
        &self,
        patch: &Path,
        cache: &HotpatchModuleCache,
    ) -> Result<JumpTable> {
        use crate::build::patch::{
            create_native_jump_table, create_wasm_jump_table, create_windows_jump_table,
        };

        let root_dir = self.root_dir();
        let base_path = self.base_path();
        let triple = &self.triple;

        // Symbols are stored differently based on the platform, so we need to handle them differently.
        // - Wasm requires the walrus crate and actually modifies the patch file
        // - windows requires the pdb crate and pdb files
        // - nix requires the object crate
        let mut jump_table = match triple.operating_system {
            OperatingSystem::Windows => create_windows_jump_table(patch, cache)?,
            _ if triple.architecture == Architecture::Wasm32 => {
                create_wasm_jump_table(patch, cache)?
            }
            _ => create_native_jump_table(patch, triple, cache)?,
        };

        // root_dir: &Path,
        //     base_path: Option<&str>,
        // Rebase the wasm binary to be relocatable once the jump table is generated
        if triple.architecture == target_lexicon::Architecture::Wasm32 {
            // Make sure we use the dir relative to the public dir, so the web can load it as a proper URL
            //
            // ie we would've shipped `/Users/foo/Projects/dioxus/target/dx/project/debug/web/public/wasm/lib.wasm`
            //    but we want to ship `/wasm/lib.wasm`
            jump_table.lib = PathBuf::from(
                "/".to_string() + base_path.unwrap_or_default().trim_start_matches('/'),
            )
            .join(jump_table.lib.strip_prefix(root_dir).unwrap())
        }

        Ok(jump_table)
    }

    /// Automatically detect the linker flavor based on the target triple and any custom linkers.
    ///
    /// This tries to replicate what rustc does when selecting the linker flavor based on the linker
    /// and triple.
    fn linker_flavor(&self) -> LinkerFlavor {
        if let Some(custom) = self.custom_linker.as_ref() {
            let name = custom.file_name().unwrap().to_ascii_lowercase();
            match name.to_str() {
                Some("lld-link") => return LinkerFlavor::Msvc,
                Some("lld-link.exe") => return LinkerFlavor::Msvc,
                Some("wasm-ld") => return LinkerFlavor::WasmLld,
                Some("ld64.lld") => return LinkerFlavor::Darwin,
                Some("ld.lld") => return LinkerFlavor::Gnu,
                Some("ld.gold") => return LinkerFlavor::Gnu,
                Some("mold") => return LinkerFlavor::Gnu,
                Some("sold") => return LinkerFlavor::Gnu,
                Some("wild") => return LinkerFlavor::Gnu,
                _ => {}
            }
        }

        match self.triple.environment {
            target_lexicon::Environment::Gnu
            | target_lexicon::Environment::Gnuabi64
            | target_lexicon::Environment::Gnueabi
            | target_lexicon::Environment::Gnueabihf
            | target_lexicon::Environment::GnuLlvm => LinkerFlavor::Gnu,
            target_lexicon::Environment::Musl => LinkerFlavor::Gnu,
            target_lexicon::Environment::Android => LinkerFlavor::Gnu,
            target_lexicon::Environment::Msvc => LinkerFlavor::Msvc,
            target_lexicon::Environment::Macabi => LinkerFlavor::Darwin,
            _ => match self.triple.operating_system {
                OperatingSystem::Darwin(_) => LinkerFlavor::Darwin,
                OperatingSystem::IOS(_) => LinkerFlavor::Darwin,
                OperatingSystem::MacOSX(_) => LinkerFlavor::Darwin,
                OperatingSystem::Linux => LinkerFlavor::Gnu,
                OperatingSystem::Windows => LinkerFlavor::Msvc,
                _ => match self.triple.architecture {
                    target_lexicon::Architecture::Wasm32 => LinkerFlavor::WasmLld,
                    target_lexicon::Architecture::Wasm64 => LinkerFlavor::WasmLld,
                    _ => LinkerFlavor::Unsupported,
                },
            },
        }
    }

    /// Select the linker to use for this platform.
    ///
    /// We prefer to use the rust-lld linker when we can since it's usually there.
    /// On macos, we use the system linker since macho files can be a bit finicky.
    ///
    /// This means we basically ignore the linker flavor that the user configured, which could
    /// cause issues with a custom linker setup. In theory, rust translates most flags to the right
    /// linker format.
    fn select_linker(&self) -> Result<PathBuf, Error> {
        if let Some(linker) = self.custom_linker.clone() {
            return Ok(linker);
        }

        let cc = match self.linker_flavor() {
            LinkerFlavor::WasmLld => self.workspace.wasm_ld(),

            // On macOS, we use the system linker since it's usually there.
            // We could also use `lld` here, but it might not be installed by default.
            //
            // Note that this is *clang*, not `lld`.
            LinkerFlavor::Darwin => self.workspace.cc(),

            // On Linux, we use the system linker since it's usually there.
            LinkerFlavor::Gnu => self.workspace.cc(),

            // On windows, instead of trying to find the system linker, we just go with the lld.link
            // that rustup provides. It's faster and more stable then reyling on link.exe in path.
            LinkerFlavor::Msvc => self.workspace.lld_link(),

            // The rest of the platforms use `cc` as the linker which should be available in your path,
            // provided you have build-tools setup. On mac/linux this is the default, but on Windows
            // it requires msvc or gnu downloaded, which is a requirement to use rust anyways.
            //
            // The default linker might actually be slow though, so we could consider using lld or rust-lld
            // since those are shipping by default on linux as of 1.86. Window's linker is the really slow one.
            //
            // https://blog.rust-lang.org/2024/05/17/enabling-rust-lld-on-linux.html
            //
            // Note that "cc" is *not* a linker. It's a compiler! The arguments we pass need to be in
            // the form of `-Wl,<args>` for them to make it to the linker. This matches how rust does it
            // which is confusing.
            LinkerFlavor::Unsupported => self.workspace.cc(),
        };

        Ok(cc)
    }

    /// Find the rlib path for a workspace crate from its captured rustc args.
    ///
    /// Extracts `--out-dir` and `-C extra-filename` from the args to construct the exact
    /// rlib filename. This is important because multiple rlibs for the same crate can coexist
    /// in the deps directory (e.g., from different dx builds that produce different `-C metadata`),
    /// and globbing would return an arbitrary one.
    fn find_rlib_for_crate(&self, crate_name: &str, rustc_args: &RustcArgs) -> Result<PathBuf> {
        // Extract --out-dir from the captured args
        let out_dir = rustc_args
            .args
            .iter()
            .zip(rustc_args.args.iter().skip(1))
            .find(|(flag, _)| *flag == "--out-dir")
            .map(|(_, dir)| PathBuf::from(dir))
            .with_context(|| format!("No --out-dir in captured rustc args for '{crate_name}'"))?;

        // Extract -C extra-filename from captured args.
        // Cargo passes this to rustc to disambiguate output filenames via metadata hash.
        // Handle all forms: `-Cextra-filename=X`, `-C extra-filename=X`, and `-C` `extra-filename=X`.
        let extra_filename = rustc_args.args.iter().enumerate().find_map(|(i, arg)| {
            arg.strip_prefix("-Cextra-filename=")
                .map(|s| s.to_string())
                .or_else(|| {
                    if arg == "-C" {
                        rustc_args.args.get(i + 1).and_then(|next| {
                            next.strip_prefix("extra-filename=").map(|s| s.to_string())
                        })
                    } else {
                        None
                    }
                })
        });

        // If we have an exact extra-filename, construct the precise rlib path.
        if let Some(extra) = &extra_filename {
            let exact = out_dir.join(format!("lib{crate_name}{extra}.rlib"));
            if exact.exists() {
                return Ok(exact);
            }
        }

        // Fallback: glob for lib<crate_name>-<hash>.rlib in the output directory.
        // Prefer the most recently modified rlib to avoid picking up stale artifacts.
        let prefix = format!("lib{crate_name}-");
        let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
        for entry in std::fs::read_dir(&out_dir)
            .with_context(|| format!("Could not read --out-dir '{}'", out_dir.display()))?
            .flatten()
        {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&prefix) && name.ends_with(".rlib") {
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            if best.as_ref().is_none_or(|(_, t)| mtime > *t) {
                                best = Some((entry.path(), mtime));
                            }
                        }
                    }
                }
            }
        }

        best.map(|(path, _)| path).with_context(|| {
            format!(
                "No rlib found for '{crate_name}' in '{}' \
                 (looked for lib{crate_name}*.rlib, extra-filename={:?})",
                out_dir.display(),
                extra_filename
            )
        })
    }

    fn rustc_wrapper_capture_mode(&self, build_mode: &BuildMode) -> &'static str {
        match build_mode {
            BuildMode::Fat => "fat",
            BuildMode::Base { run: true } => "base-run",
            BuildMode::Base { run: false } => "base",
            BuildMode::Thin { .. } => "thin",
        }
    }

    pub fn rustc_wrapper_scope_dir_name(&self, build_mode: &BuildMode) -> Result<String> {
        #[derive(Debug, Serialize)]
        struct RustcWrapperScope {
            version: u8,
            capture_mode: &'static str,
            bundle: String,
            triple: String,
            profile: String,
            package: String,
            main_target: String,
            executable_type: String,
            rustc_version: String,
            features: Vec<String>,
            all_features: bool,
            rustflags: Vec<String>,
            extra_cargo_args: Vec<String>,
            extra_rustc_args: Vec<String>,
        }

        let scope = RustcWrapperScope {
            version: 1,
            capture_mode: self.rustc_wrapper_capture_mode(build_mode),
            bundle: self.bundle.to_string(),
            triple: self.triple.to_string(),
            profile: self.profile.clone(),
            package: self.package.clone(),
            main_target: self.main_target.clone(),
            executable_type: format!("{:?}", self.executable_type()),
            rustc_version: self.workspace.rustc_version.clone(),
            features: self.features.clone(),
            all_features: self.all_features,
            rustflags: self.rustflags.flags.clone(),
            extra_cargo_args: self.extra_cargo_args.clone(),
            extra_rustc_args: self.extra_rustc_args.clone(),
        };

        let encoded =
            serde_json::to_vec(&scope).context("Failed to serialize rustc wrapper scope")?;
        let mut hasher = Sha256::new();
        hasher.update(encoded);
        let scope_hash = format!("{:x}", hasher.finalize());
        Ok(format!(
            "{}-{}-{}-{}",
            self.tip_crate_name(),
            self.triple,
            self.profile,
            &scope_hash[..16]
        ))
    }
}
