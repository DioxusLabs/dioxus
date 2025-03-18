use anyhow::Context;
use cargo_metadata::camino::Utf8PathBuf;
use futures::{SinkExt, StreamExt};
use itertools::Itertools;
use notify::{
    event::{DataChange, ModifyKind},
    Watcher,
};
use object::write::Object;
use serde::Deserialize;
use std::{collections::HashMap, env, ffi::OsStr, path::PathBuf, process::Stdio, time::SystemTime};
use subsecond_cli_support::create_jump_table;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::TcpListener,
    process::{Child, Command},
    time::Instant,
};
use tokio_tungstenite::WebSocketStream;

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
    let src_folder = subsecond_folder().join("subsecond-harness/src/");
    let main_rs = src_folder.join("main.rs");

    // Modify the main.rs mtime so we skip "fresh" builds
    // Basically `touch main.rs` in the directory
    std::fs::File::open(&main_rs)?.set_modified(SystemTime::now())?;

    // Perform the initial build
    let epoch = SystemTime::UNIX_EPOCH;
    let now = std::time::Instant::now();
    println!("Starting build...");
    let result = initial_build().await?;
    println!(
        "Initial build: {:?} -> {}",
        now.elapsed(),
        &result.output_location,
    );

    // copy the exe and give it a "fat" name. todo: wipe the ld entry that points to `/deps`
    let exe = &result.output_location;
    let fat_exe = exe.with_file_name(format!(
        "fatharness-{}",
        epoch.elapsed().unwrap().as_millis()
    ));
    std::fs::copy(&exe, &fat_exe).unwrap();

    // Launch the fat exe. We'll overwrite the slim exe location, so this prevents the app from bugging out
    let app = Command::new(&fat_exe).kill_on_drop(true).spawn()?;

    // Wait for the websocket to come up
    let mut websocket = wait_for_ws(9393).await?;

    // don't log if the screen has been taken over - important for tui apps
    let should_log = rust_log_enabled();

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

        // Assemble the jump table of redirected addresses
        // todo: keep track of this and merge it over time
        let jump_table = create_jump_table(fat_exe.as_std_path(), output_temp.as_std_path());

        websocket
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                bincode::serialize(&jump_table).unwrap().into(),
            ))
            .await?;

        if should_log {
            println!("Patching complete in {}ms", started.elapsed().as_millis())
        }
    }

    drop(app);

    Ok(())
}

struct FsWatcher {
    _watcher: notify::RecommendedWatcher,
    files: HashMap<PathBuf, String>,
    rx: futures_channel::mpsc::UnboundedReceiver<Result<notify::Event, notify::Error>>,
}

impl FsWatcher {
    fn watch(src_folder: PathBuf) -> anyhow::Result<Self> {
        let (tx, rx) = futures_channel::mpsc::unbounded();
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

        Ok(FsWatcher {
            files,
            rx,
            _watcher: watcher,
        })
    }

    /// Check if the file has changed and update the internal state
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

async fn wait_for_ws(port: u16) -> anyhow::Result<WebSocketStream<tokio::net::TcpStream>> {
    let port = port;
    let addr = format!("127.0.0.1:{}", port);
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    let (conn, sock) = listener.accept().await?;
    let socket = tokio_tungstenite::accept_async(conn).await?;
    Ok(socket)
}

/// Store the linker args in a file for the main process to read.
async fn link(action: String) -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();

    // Write the linker args to a file for the main process to read
    std::fs::write(
        subsecond_folder().join("data").join("link.txt"),
        args.join("\n"),
    )?;

    match action.as_str() {
        // Write a dummy object file to the output file to satisfy rust when it tries to strip the symbols
        "patch" => {
            let out = args.iter().position(|arg| arg == "-o").unwrap();
            let out_file = args[out + 1].clone();
            let dummy_object_file = Object::new(
                object::BinaryFormat::MachO,
                object::Architecture::Aarch64,
                object::Endianness::Big,
            );
            let bytes = dummy_object_file.write().unwrap();
            std::fs::write(out_file, bytes)?;
        }

        // Actually link the object file. todo: figure out which linker we should be using
        "link" => {}

        _ => anyhow::bail!("Unknown action: {}", action),
    }

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
        .arg("subsecond-harness")
        .arg("--bin")
        .arg("subsecond-harness")
        .arg("--profile")
        .arg("subsecond-dev")
        .arg("--message-format")
        .arg("json-diagnostic-rendered-ansi")
        .arg("--verbose")
        .arg("--")
        // these args are required to prevent DCE, save intermediates, and print the link args for future usage
        // -all_load ensures all statics get bubbled out
        // -link-dead-code prevents the flag `-Wl,-dead_strip` from being passed
        // -save-temps ensures the intermediates are saved so we can use them for comparsions
        //
        // todo: delete the temps
        .arg("-Clink-arg=-Wl,-all_load")
        .arg("-Clink-dead-code")
        .arg("-Csave-temps=true")
        // we capture the link args, but eventually we should actually just use ourselves as the linker since that's more robust
        .arg("--print")
        .arg("link-args")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .current_dir(workspace_dir())
        .spawn()?;

    run_cargo_output(inital_build, rust_log_enabled()).await
}

fn rust_log_enabled() -> bool {
    match env::var("RUST_LOG").as_deref() {
        Ok("debug") => true,
        _ => false,
    }
}

async fn fast_build(original: &CargoOutputResult) -> anyhow::Result<Utf8PathBuf> {
    let fast_build = Command::new(original.direct_rustc[0].clone())
        .args(original.direct_rustc[1..].iter())
        .arg("-C")
        .arg(format!(
            "linker={}",
            std::env::current_exe().unwrap().display()
        ))
        .env("HOTRELOAD_LINK", "patch")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(workspace_dir())
        .spawn()?;

    let output = run_cargo_output(fast_build, rust_log_enabled()).await?;

    let object_files = output
        .link_args
        .iter()
        .filter(|arg| arg.ends_with(".rcgu.o"))
        .sorted()
        .collect::<Vec<_>>();

    let output_location = original.output_location.with_file_name(format!(
        "patch-{}",
        SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis()
    ));

    // todo: we should throw out symbols that we don't need and/or assemble them manually
    let res = Command::new("cc")
        .args(object_files)
        .arg("-dylib")
        .arg("-Wl,-undefined,dynamic_lookup")
        .arg("-Wl,-export_dynamic")
        .arg("-arch")
        .arg("arm64")
        .arg("-o")
        .arg(&output_location)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let errs = String::from_utf8_lossy(&res.stderr);
    if !errs.is_empty() {
        println!("errs: {errs}");
    }

    Ok(output_location)
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

fn workspace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../")
        .canonicalize()
        .unwrap()
}

/// Folder representing dioxus/packages/subsecond
fn subsecond_folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../")
        .canonicalize()
        .unwrap()
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
