use std::{
    collections::HashMap,
    ffi::{CString, OsStr},
    path::PathBuf,
    process::Stdio,
    ptr::{self, null_mut},
    time::SystemTime,
};

use anyhow::Context;
use cargo_metadata::camino::Utf8PathBuf;
use futures::StreamExt;
use itertools::Itertools;
use notify::{
    event::{DataChange, ModifyKind},
    Watcher,
};
use object::write::Object;
use serde::Deserialize;
use subsecond_cli_support::create_jump_table;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    process::{Child, Command},
    time::Instant,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Go through the linker if we need to
    if let Ok(action) = std::env::var("HOTRELOAD_LINK") {
        return link(action).await;
    }

    hotreload_loop().await
}

/// The main loop of the hotreload process
///
/// 1. Create initial "fat" build
/// 2. Identify hotpoints from the incrementals. We ignore dependency hotpoints for now, but eventually might want to aggregate workspace deps together.
/// 3. Wait for changes to the main.rs file
/// 4. Perform a "fast" build
/// 5. Diff the object files, walking relocations, preserving local statics
/// 6. Create a minimal patch file to load into the process, including the changed symbol list
/// 7. Pause the process with lldb, run the "hotfn_load_binary_patch" command and then continue
/// 8. Repeat
async fn hotreload_loop() -> anyhow::Result<()> {
    // Save the state of the rust files
    let src_folder = subsecond_folder().join("subsecond-harness/src");
    let main_rs = PathBuf::from(src_folder.join("main.rs"));

    // Modify the main.rs mtime so we skip "fresh" builds
    // Basically `touch main.rs` in the directory
    std::fs::File::open(&main_rs)?.set_modified(SystemTime::now())?;

    // Perform the initial build
    let epoch = std::time::SystemTime::UNIX_EPOCH;
    let now = std::time::Instant::now();
    let result = initial_build().await?;
    println!(
        "Initial build: {:?} -> {}",
        now.elapsed(),
        &result.output_location,
    );

    // copy the exe and give it a "fat" name
    let exe = &result.output_location;
    let fat_exe = exe.with_file_name(format!(
        "fatharness-{}",
        epoch.elapsed().unwrap().as_millis()
    ));
    std::fs::copy(&exe, &fat_exe).unwrap();

    // Launch the fat exe. We'll overwrite the slim exe location, so this prevents the app from bugging out
    // todo - we can launch exe with lldb directly to force aslr off. This won't work for wasm though
    // let app = Command::new(&fat_exe)
    //     .stdin(Stdio::piped())
    //     .kill_on_drop(true)
    //     .spawn()?;

    let mut pid = 0;
    unsafe {
        let program_c = std::ffi::CString::new(fat_exe.as_os_str().to_str().unwrap()).unwrap();
        let mut attr: libc::posix_spawnattr_t = unsafe { std::mem::zeroed() };
        let ret = libc::posix_spawnattr_init(&mut attr);
        if ret != 0 {
            panic!("posix_spawnattr_init failed");
        }

        // Use current environment
        extern "C" {
            static environ: *const *const libc::c_char;
        }

        // Convert args to CStrings
        let args: Vec<String> = vec![];
        let mut args_vec: Vec<CString> = Vec::with_capacity(args.len() + 1);
        args_vec.push(program_c.clone());
        for arg in args {
            args_vec.push(CString::new(arg)?);
        }

        // Create null-terminated array of pointers to args
        let mut args_ptr: Vec<*const libc::c_char> =
            args_vec.iter().map(|arg| arg.as_ptr()).collect();
        args_ptr.push(ptr::null());

        const POSIX_SPAWN_DISABLE_ASLR: libc::c_int = 0x0100;

        // Set the flag to disable ASLR
        let ret = libc::posix_spawnattr_setflags(
            &mut attr,
            (POSIX_SPAWN_DISABLE_ASLR) as _,
            // (POSIX_SPAWN_DISABLE_ASLR | libc::POSIX_SPAWN_SETEXEC) as _,
        );
        if ret != 0 {
            libc::posix_spawnattr_destroy(&mut attr);
            panic!("posix_spawnattr_setflags failed");
        }

        let mut fileactions: libc::posix_spawn_file_actions_t = null_mut();
        let ret = libc::posix_spawn_file_actions_init(&mut fileactions);

        println!("Bout to spawn with attr: {:?}", attr);
        libc::posix_spawn(
            &mut pid,
            program_c.as_ptr(),
            &fileactions,
            &attr,
            args_ptr.as_ptr() as *const *mut libc::c_char,
            environ as *const _,
        );

        println!("Spawning process with pid: {}", pid);
    };

    // Launch with lldb, disabling ASLR
    let mut lldb = Command::new("lldb")
        // .arg("-o")
        // .arg("run")
        .arg("-p")
        .arg(format!("{}", pid))
        // .arg(format!("{}", app.id().unwrap()))
        .arg(&fat_exe)
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // Immediately resume the process
    lldb.stdin
        .as_mut()
        .unwrap()
        .write_all(b"process continue\n")
        .await?;

    // don't log if the screen has been taken over - important for tui apps
    let should_log = !crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);

    // Watch the source folder for changes
    let mut watcher = FsWatcher::watch(src_folder)?;

    while let Some(Ok(event)) = watcher.rx.next().await {
        if event.kind != notify::EventKind::Modify(ModifyKind::Data(DataChange::Content)) {
            continue;
        }

        if !watcher.file_changed(event.paths.first().unwrap()) {
            continue;
        }

        if should_log {
            println!("Fast reloading... ");
        }

        let started = Instant::now();
        let Ok(output_temp) = fast_build(&result).await else {
            continue;
        };

        let jump_table = create_jump_table(fat_exe.as_std_path(), output_temp.as_std_path());
        let jump_table_path = subsecond_folder().join("data").join("jump_table.bin");
        std::fs::write(&jump_table_path, bincode::serialize(&jump_table).unwrap()).unwrap();

        // Pause the process with lldb, run the "hotfn_load_binary_patch" command and then continue
        lldb.stdin
            .as_mut()
            .unwrap()
            .write_all(
                format!(
                    "process interrupt\nexpr (void) hotfn_load_binary_patch(\"{}\", \"{}\")\ncontinue\n",
                    output_temp,
                    jump_table_path.display(),

                )
                .as_bytes(),
            )
            .await?;

        if should_log {
            println!("Patching complete in {}ms", started.elapsed().as_millis())
        }
    }

    drop(lldb);

    Ok(())
}

struct FsWatcher {
    files: HashMap<PathBuf, String>,
    rx: futures_channel::mpsc::UnboundedReceiver<Result<notify::Event, notify::Error>>,
}

impl FsWatcher {
    fn watch(src_folder: PathBuf) -> anyhow::Result<Self> {
        let (tx, mut rx) = futures_channel::mpsc::unbounded();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                _ = tx.unbounded_send(res);
            })?;
        let mut files = HashMap::new();
        for entry in walkdir::WalkDir::new(src_folder) {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() || path.extension() != Some(OsStr::new("rs")) {
                continue;
            }
            files.insert(path.to_path_buf(), std::fs::read_to_string(&path).unwrap());
            watcher.watch(&path, notify::RecursiveMode::NonRecursive)?;
        }

        Ok(FsWatcher { files, rx })
    }

    fn file_changed(&mut self, path: &PathBuf) -> bool {
        if let Some(contents) = self.files.get_mut(path) {
            let new_contents = std::fs::read_to_string(&path).unwrap();
            if new_contents == *contents {
                return false;
            }
            *contents = new_contents;
            return true;
        }

        false
    }
}

/// Store the linker args in a file for the main process to read.
async fn link(action: String) -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();

    std::fs::write(
        subsecond_folder().join("data").join("link.txt"),
        args.join("\n"),
    )?;

    let out = args.iter().position(|arg| arg == "-o").unwrap();
    let out_file = args[out + 1].clone();
    let dummy_object_file = Object::new(
        object::BinaryFormat::MachO,
        object::Architecture::Aarch64,
        object::Endianness::Big,
    );
    let bytes = dummy_object_file.write().unwrap();
    std::fs::write(out_file, bytes)?;

    Ok(())
}

async fn initial_build() -> anyhow::Result<CargoOutputResult> {
    // Perform the initial build and print out the link arguments. Don't strip dead code and preserve temp files.
    // This results in a "fat" executable that we can bind to
    //
    // todo: clean up the temps manually
    let inital_build = Command::new("cargo")
        .arg("rustc")
        .arg("--package")
        .arg("harness")
        .arg("--bin")
        .arg("harness")
        .arg("--profile")
        .arg("hotreload")
        .arg("--message-format")
        .arg("json-diagnostic-rendered-ansi")
        .arg("--verbose")
        .arg("--")
        // these args are required to prevent DCE, save intermediates, and print the link args for future usage
        .arg("-Clink-arg=-Wl,-all_load")
        .arg("-Clink-dead-code")
        .arg("-Csave-temps=true")
        .arg("--print")
        .arg("link-args")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    run_cargo_output(inital_build, false).await
}

async fn fast_build(original: &CargoOutputResult) -> anyhow::Result<Utf8PathBuf> {
    let fast_build = Command::new(original.direct_rustc[0].clone())
        .args(original.direct_rustc[1..].iter())
        .arg("-C")
        .arg(format!(
            "linker={}",
            std::env::current_exe().unwrap().display()
        ))
        .env("HOTRELOAD_LINK", "reload")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let output = run_cargo_output(fast_build, false).await?;

    let object_files = output
        .link_args
        .iter()
        .filter(|arg| arg.ends_with(".rcgu.o"))
        .sorted()
        .collect::<Vec<_>>();

    // println!("Fast link objects: {:?}", object_files);

    let epoch = std::time::SystemTime::UNIX_EPOCH;
    let target_loc = original
        .output_location
        .with_file_name(format!("patch-{}", epoch.elapsed().unwrap().as_millis()));

    // println!("target_loc: {target_loc:?}");

    // we should throw out symbols that we don't need and/or assemble them manually
    let res = Command::new("cc")
        .args(object_files)
        .arg("-dylib")
        .arg("-Wl,-undefined,dynamic_lookup")
        .arg("-Wl,-export_dynamic")
        // .arg("-Wl,-unexported_symbol,_main")
        .arg("-arch")
        .arg("arm64")
        .arg("-o")
        .arg(&target_loc)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    let errs = String::from_utf8_lossy(&res.stderr);
    if !errs.is_empty() {
        println!("errs: {errs}");
    }

    // println!("Fast link args: {:?}", output.link_args);
    // .arg("-undefined")
    // .arg("dynamic_lookup")
    // .arg("-Wl,-export_dynamic")
    // .arg("-Wl,-exported_symbol,__ZN7harness3app17h0df796a0810dae7cE")
    // .arg("-Wl,-exported_symbol,___ZN7harness3app17h0df796a0810dae7cE")
    // .arg("-Wl,-exported_symbol,_ZN7harness3app17h0df796a0810dae7cE")
    // .arg("-Wl,-all_load")
    //         // -O0 ? supposedly faster
    //         // -reproducible - even better?
    //         // -exported_symbol and friends - could help with dead-code stripping
    //         // -e symbol_name - for setting the entrypoint
    //         // -keep_relocs ?
    // .arg("-Clink-dead-code")
    // .arg("-Wl,-unexported_symbol,_main")
    // .arg("-dead_strip") // maybe?

    Ok(target_loc)
}

/// Folder representing dioxus/packages/subsecond
fn subsecond_folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../")
}

struct CargoOutputResult {
    output_location: Utf8PathBuf,
    direct_rustc: Vec<String>,
    link_args: Vec<String>,
}

async fn run_cargo_output(
    mut child: Child,
    should_render: bool,
) -> anyhow::Result<CargoOutputResult> {
    let stdout = tokio::io::BufReader::new(child.stdout.take().unwrap());
    let stderr = tokio::io::BufReader::new(child.stderr.take().unwrap());
    let mut output_location = None;
    let mut stdout = stdout.lines();
    let mut stderr = stderr.lines();

    let mut link_args = vec![];
    let mut direct_rustc = vec![];

    loop {
        use cargo_metadata::Message;

        let line = tokio::select! {
            Ok(Some(line)) = stdout.next_line() => line,
            Ok(Some(line)) = stderr.next_line() => line,
            else => break,
        };

        let mut messages = Message::parse_stream(std::io::Cursor::new(line));

        loop {
            let message = match messages.next() {
                Some(Ok(message)) => message,
                None => break,
                other => {
                    println!("other: {other:?}");
                    break;
                }
            };

            match message {
                Message::CompilerArtifact(artifact) => {
                    if let Some(i) = artifact.executable {
                        output_location = Some(i)
                    }
                }
                Message::CompilerMessage(compiler_message) => {
                    if let Some(rendered) = &compiler_message.message.rendered {
                        if should_render {
                            println!("rendered: {rendered}");
                        }
                    }
                }
                Message::BuildScriptExecuted(_build_script) => {}
                Message::BuildFinished(build_finished) => {
                    if !build_finished.success {
                        // assuming we received a message from the compiler, so we can exit
                        anyhow::bail!("Build failed");
                    }
                }
                Message::TextLine(word) => {
                    if word.trim().starts_with("Running ") {
                        // trim everyting but the contents between the quotes
                        let args = word
                            .trim()
                            .trim_start_matches("Running `")
                            .trim_end_matches('`');

                        direct_rustc = shell_words::split(args).unwrap();
                    }

                    if word.trim().starts_with("env") {
                        link_args = shell_words::split(&word).unwrap();
                    }

                    #[derive(Debug, Deserialize)]
                    struct RustcArtifact {
                        artifact: PathBuf,
                        emit: String,
                    }

                    if let Ok(artifact) = serde_json::from_str::<RustcArtifact>(&word) {
                        if artifact.emit == "link" {
                            output_location =
                                Some(Utf8PathBuf::from_path_buf(artifact.artifact).unwrap());
                        }
                    }

                    if should_render {
                        println!("text: {word}")
                    }
                }
                _ => {}
            }
        }
    }

    let output_location =
        output_location.context("Failed to find output location. Build must've failed.")?;

    Ok(CargoOutputResult {
        output_location,
        link_args,
        direct_rustc,
    })
}

/// Move all previous object files to "incremental-old" and all new object files to "incremental-new"
fn cache_incrementals(object_files: &[&String]) {
    let old = subsecond_folder().join("data").join("incremental-old");
    let new = subsecond_folder().join("data").join("incremental-new");

    // Remove the old incremental-old directory if it exists
    _ = std::fs::remove_dir_all(&old);

    // Rename incremental-new to incremental-old if it exists. Faster than moving all the files
    _ = std::fs::rename(&new, &old);

    // Create the new incremental-new directory to place the outputs in
    std::fs::create_dir_all(&new).unwrap();

    // Now drop in all the new object files
    for o in object_files.iter() {
        if !o.ends_with(".rcgu.o") {
            continue;
        }

        let path = PathBuf::from(o);
        std::fs::copy(&path, new.join(path.file_name().unwrap())).unwrap();
    }
}

#[test]
fn where_are_we() {
    let root = subsecond_folder();
    println!("{root:?}");
}
