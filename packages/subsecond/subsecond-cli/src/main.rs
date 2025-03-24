use anyhow::Context;
use cargo_metadata::camino::Utf8PathBuf;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use itertools::Itertools;
use notify::{
    event::{DataChange, ModifyKind},
    Watcher,
};
use object::{write::Object, Architecture};
use serde::Deserialize;
use std::{collections::HashMap, env, ffi::OsStr, path::PathBuf, process::Stdio, time::SystemTime};
use subsecond_cli_support::{create_jump_table, wasm::move_func_initiailizers};
use target_lexicon::{Environment, Triple};
use tokio::{
    io::AsyncBufReadExt,
    net::TcpListener,
    process::{Child, Command},
    time::Instant,
};
use tokio_tungstenite::WebSocketStream;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Go through the linker if we need to
    if let Ok(action) = std::env::var("HOTRELOAD_LINK") {
        return link(action).await;
    }

    hotreload_loop().await
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(long)]
    target: Option<String>,
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
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let target: Triple = args
        .target
        .map(|t| t.parse().unwrap())
        .unwrap_or_else(|| Triple::host());

    // Save the state of the rust files
    let src_folder = subsecond_folder().join("subsecond-harness/src/");
    let main_rs = src_folder.join("main.rs");

    // Modify the main.rs mtime so we skip "fresh" builds
    // Basically `touch main.rs` in the directory
    std::fs::File::open(&main_rs)?.set_modified(SystemTime::now())?;

    // Perform the initial build
    let epoch = SystemTime::UNIX_EPOCH;
    let now = std::time::Instant::now();
    tracing::debug!("Starting build for target {target:?}...");
    let result = initial_build(&target).await?;
    tracing::debug!(
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
    let app = launch_app(&fat_exe, &target)?;

    // Wait for the websocket to come up
    let mut client = wait_for_ws(9393, &target).await?;

    // Watch the source folder for changes
    let mut watcher = FsWatcher::watch(src_folder)?;

    while let Some(Ok(event)) = watcher.rx.next().await {
        if event.kind != notify::EventKind::Modify(ModifyKind::Data(DataChange::Content)) {
            continue;
        }

        if !watcher.file_changed(event.paths.first().unwrap()) {
            continue;
        }

        tracing::info!("Fast reloading... ");

        let started = Instant::now();
        let output_temp = match fast_build(
            &result,
            &target,
            client
                .as_ref()
                .map(|s| s.aslr_reference)
                .unwrap_or_default(),
        )
        .await
        {
            Ok(output_temp) => output_temp,
            Err(e) => {
                tracing::warn!("Fast build failed: {e}");
                continue;
            }
        };

        // Assemble the jump table of redirected addresses
        // todo: keep track of this and merge it over time
        let jump_table =
            create_jump_table(fat_exe.as_std_path(), output_temp.as_std_path(), &target).unwrap();

        if let Some(client) = client.as_mut() {
            client
                .socket
                .send(tokio_tungstenite::tungstenite::Message::Binary(
                    bincode::serialize(&jump_table).unwrap().into(),
                ))
                .await?;
        }

        if target.architecture == target_lexicon::Architecture::Wasm32 {
            let _ = std::fs::copy(
                output_temp.as_std_path(),
                static_folder().join(output_temp.file_name().unwrap()),
            );
        }

        tracing::info!("Patching complete in {}ms", started.elapsed().as_millis())
    }

    drop(app);

    Ok(())
}

fn launch_app(fat_exe: &Utf8PathBuf, target: &Triple) -> Result<Child, anyhow::Error> {
    let app = match target.architecture {
        target_lexicon::Architecture::Wasm32 => {
            info!("Serving wasm at http://127.0.0.1:9393");
            Command::new("python3")
                .current_dir(static_folder())
                .arg("-m")
                .arg("http.server")
                .arg("9394")
                .arg("--directory")
                .arg(".")
                .kill_on_drop(true)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        }
        _ => Command::new(fat_exe).kill_on_drop(true).spawn()?,
    };

    Ok(app)
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

struct WsClient {
    aslr_reference: u64,
    socket: WebSocketStream<tokio::net::TcpStream>,
}

async fn wait_for_ws(port: u16, target: &Triple) -> anyhow::Result<Option<WsClient>> {
    if target.architecture == target_lexicon::Architecture::Wasm32 {
        return Ok(None);
    }

    let addr = format!("127.0.0.1:{}", port);
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    let (conn, _sock) = listener.accept().await?;
    let mut socket = tokio_tungstenite::accept_async(conn).await?;
    let msg = socket.next().await.unwrap()?;
    let aslr_reference = msg.into_text().unwrap().parse().unwrap();

    Ok(Some(WsClient {
        aslr_reference,
        socket,
    }))
}

/// Store the linker args in a file for the main process to read.
async fn link(action: String) -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();

    // Write the linker args to a file for the main process to read
    std::fs::write(link_args_file(), args.join("\n"))?;

    match action.as_str() {
        // Actually link the object file. todo: figure out which linker we should be using
        "link" => {}

        // // Run the wasm-link process
        // // This involves finding all the non-describe functions and calling --export on them
        // "wasm-link" => {}

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

        _ => anyhow::bail!("Unknown action: {}", action),
    }

    Ok(())
}

fn link_args_file() -> PathBuf {
    subsecond_folder().join("data").join("link.txt")
}

async fn initial_build(target: &Triple) -> anyhow::Result<CargoOutputResult> {
    // Perform the initial build and print out the link arguments. Don't strip dead code and preserve temp files.
    // This results in a "fat" executable that we can bind to
    //
    // todo: clean up the temps manually
    let mut build = Command::new("cargo");

    build
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
        .arg("--target")
        .arg(target.to_string());

    match target.architecture {
        target_lexicon::Architecture::Wasm32 => {
            build.arg("--features").arg("web");
        }
        _ => {}
    }

    // these args are required to prevent DCE, save intermediates, and print the link args for future usage
    // -all_load ensures all statics get bubbled out
    // -link-dead-code prevents the flag `-Wl,-dead_strip` from being passed
    // -save-temps ensures the intermediates are saved so we can use them for comparsions
    build
        .arg("--")
        .arg("-Csave-temps=true")
        .arg("-Clink-dead-code");

    match target.architecture {
        // usually just ld64 - uses your `cc`
        target_lexicon::Architecture::Aarch64(_) => {
            build.arg("-Clink-arg=-Wl,-all_load");
        }

        // /Users/jonkelley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/wasm-ld
        target_lexicon::Architecture::Wasm32 => {
            // we want "all-load", adjustable ifunc table,
            build.arg("-Clink-arg=--no-gc-sections");
            build.arg("-Clink-arg=--growable-table");
            build.arg("-Clink-arg=--whole-archive");
            build.arg("-Clink-arg=--export-table");
            build.arg("-Clink-arg=--export-memory");
            build.arg("-Clink-arg=--emit-relocs");
        }
        _ => {}
    }

    // we capture the link args, but eventually we should actually just use ourselves as the linker since that's more robust
    build
        .arg("--print")
        .arg("link-args")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .current_dir(workspace_dir());

    let build = build.spawn()?;

    let out = run_cargo_output(build, rust_log_enabled()).await?;

    if target.architecture == target_lexicon::Architecture::Wasm32 {
        _ = std::fs::remove_dir_all(static_folder());

        let test_data_folder = wasm_data_folder();
        let _ = std::fs::create_dir_all(&test_data_folder);
        let _ = std::fs::copy(
            out.output_location.as_std_path(),
            test_data_folder.join("pre-bindgen.wasm"),
        );

        let unprocessed = std::fs::read(out.output_location.as_std_path())?;
        let all_exported_bytes =
            subsecond_cli_support::wasm::prepare_base_module(&unprocessed).unwrap();
        let processed = test_data_folder.join("processed.wasm");
        std::fs::write(&processed, all_exported_bytes)?;

        let bind = Command::new("wasm-bindgen")
            .arg("--target")
            .arg("web")
            .arg("--no-typescript")
            .arg("--out-dir")
            .arg(static_folder())
            .arg("--out-name")
            .arg("main")
            .arg("--no-demangle")
            .arg("--keep-lld-exports")
            .arg("--keep-debug")
            .arg(&processed)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .current_dir(workspace_dir())
            .output()
            .await?;

        let err = String::from_utf8(bind.stderr).unwrap();
        if !err.is_empty() {
            tracing::error!("err: {err}");
        }

        let _ = std::fs::copy(
            static_folder().join("main_bg.wasm"),
            test_data_folder.join("post-bindgen.wasm"),
        );

        let index = include_str!("./index.html");
        std::fs::write(static_folder().join("index.html"), index).unwrap();
    }

    Ok(out)
}

fn wasm_data_folder() -> PathBuf {
    subsecond_folder().join("data").join("wasm")
}

fn static_folder() -> PathBuf {
    subsecond_folder().join("subsecond-harness").join("static")
}

fn rust_log_enabled() -> bool {
    match env::var("RUST_LOG").as_deref() {
        Ok("debug") => true,
        _ => false,
    }
}

async fn fast_build(
    original: &CargoOutputResult,
    target: &Triple,
    aslr_reference: u64,
) -> anyhow::Result<Utf8PathBuf> {
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

    tracing::info!("fast_build output: {output:#?}");

    let link_args = std::fs::read_to_string(link_args_file())?;
    let mut object_files = link_args
        .lines()
        .filter(|arg| arg.ends_with(".rcgu.o"))
        .sorted()
        .map(|arg| PathBuf::from(arg))
        .collect::<Vec<_>>();

    // copy incrementals to the data folder
    if target.architecture == target_lexicon::Architecture::Wasm32 {
        let test_data_folder = wasm_data_folder().join("incrementals");
        let _ = std::fs::create_dir_all(&test_data_folder);
        for object in object_files.iter() {
            let dest = test_data_folder.join(object.file_name().unwrap());
            std::fs::copy(object, dest)?;
        }
    }

    // tracing::info!("object files: {object_files:#?}");

    // on wasm we'll need to add in some symbols that resolve to the ifunc table
    // unfortunately we can't quite call functions from main directly so we need to go through the ifunc system
    // I *think* we can just import them
    let resolved = subsecond_cli_support::resolve_undefined(
        &original.output_location.as_std_path(),
        &object_files,
        target,
        aslr_reference,
    )
    .unwrap();
    let syms = subsecond_folder().join("data").join("syms.o");
    std::fs::write(&syms, resolved).unwrap();
    if target.architecture != target_lexicon::Architecture::Wasm32 {
        object_files.push(syms);
    }

    let output_location = original
        .output_location
        .with_file_name(format!(
            "patch-{}",
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis()
        ))
        .with_extension(match target.architecture {
            target_lexicon::Architecture::Wasm32 => "wasm",
            _ => "",
        });

    let res = match target.architecture {
        // usually just ld64 - uses your `cc`
        target_lexicon::Architecture::Aarch64(_) => {
            // todo: we should throw out symbols that we don't need and/or assemble them manually
            Command::new("cc")
                .args(object_files)
                .arg("-Wl,-dylib")
                // .arg("-Wl,-undefined,dynamic_lookup")
                // .arg("-Wl,-export_dynamic")
                .arg("-arch")
                .arg("arm64")
                .arg("-o")
                .arg(&output_location)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?
        }
        target_lexicon::Architecture::Wasm32 => {
            Command::new(wasm_ld().await.unwrap())
                .args(object_files)
                .arg("--import-memory")
                .arg("--import-table")
                .arg("--growable-table")
                .arg("--export")
                .arg("main")
                .arg("--export-all")
                // .arg("--export=__heap_base")
                // .arg("--export=__data_end")
                // .arg("-z")
                // .arg("stack-size=1048576")
                .arg("--stack-first")
                .arg("--allow-undefined")
                .arg("--no-demangle")
                .arg("--no-entry")
                .arg("-o")
                .arg(&output_location)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?
        }
        _ => todo!(),
    };

    let errs = String::from_utf8_lossy(&res.stderr);
    if !errs.is_empty() {
        tracing::error!("errs: {errs}");
    }

    if target.architecture == target_lexicon::Architecture::Wasm32 {
        let out_bytes = std::fs::read(&output_location).unwrap();
        let res_ = move_func_initiailizers(&out_bytes).unwrap();
        std::fs::write(&output_location, res_).unwrap();
    }

    Ok(output_location)
}

#[derive(Debug)]
struct CargoOutputResult {
    output_location: Utf8PathBuf,
    direct_rustc: Vec<String>,
    link_args: Vec<String>,
    table_size: usize,
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
                    tracing::trace!("other: {other:?}");
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
                        tracing::trace!("rendered: {rendered}");
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

                    tracing::trace!("text: {word}")
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
        table_size: 0,
    })
}

async fn wasm_ld() -> anyhow::Result<PathBuf> {
    Ok("/Users/jonkelley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/wasm-ld".into())
    // let root = Command::new("rustc")
    //     .arg("--print")
    //     .arg("--sysroot")
    //     .output()
    //     .await?;
    // let root = String::from_utf8(root.stdout)?;
    // let root = PathBuf::from(root.trim());
    // Ok(root.join("lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/wasm-ld"))
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
