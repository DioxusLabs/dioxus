use clap::Parser;
use std::path::PathBuf;
use wasm_split_cli::SplitModule;

fn main() {
    tracing_subscriber::fmt()
        .without_time()
        .compact()
        .with_env_filter("debug,walrus=info")
        .init();

    match Commands::parse() {
        Commands::Split(split_args) => split(split_args),
        Commands::Validate(validate_args) => validate(validate_args),
    }
}

#[derive(Parser)]
enum Commands {
    /// Split a wasm module into multiple chunks
    #[clap(name = "split")]
    Split(SplitArgs),

    /// Validate the main module of a wasm module
    #[clap(name = "validate")]
    Validate(ValidateArgs),
}

#[derive(Parser)]
struct SplitArgs {
    /// The wasm module emitted by rustc
    original: PathBuf,

    /// The wasm module emitted by wasm-bindgen
    bindgened: PathBuf,

    /// The output *directory* to write the split wasm files to
    out_dir: PathBuf,
}

fn split(args: SplitArgs) {
    let original = std::fs::read(&args.original).expect("failed to read input file");
    let bindgened = std::fs::read(&args.bindgened).expect("failed to read input file");

    _ = std::fs::remove_dir_all(&args.out_dir);
    std::fs::create_dir_all(&args.out_dir).expect("failed to create output dir");

    tracing::info!("Building split module");

    let module = wasm_split_cli::Splitter::new(&original, &bindgened).unwrap();

    let mut chunks = module.emit().unwrap();

    // Write out the main module
    tracing::info!(
        "Writing main module to {}",
        args.out_dir.join("main.wasm").display()
    );
    std::fs::write(args.out_dir.join("main.wasm"), &chunks.main.bytes).unwrap();

    // Write the js module
    std::fs::write(
        args.out_dir.join("__wasm_split.js"),
        emit_js(&chunks.chunks, &chunks.modules),
    )
    .expect("failed to write js module");

    for (idx, chunk) in chunks.chunks.iter().enumerate() {
        tracing::info!(
            "Writing chunk {} to {}",
            idx,
            args.out_dir
                .join(format!("chunk_{}_{}.wasm", idx, chunk.module_name))
                .display()
        );
        std::fs::write(
            args.out_dir
                .join(format!("chunk_{}_{}.wasm", idx, chunk.module_name)),
            &chunk.bytes,
        )
        .expect("failed to write chunk");
    }

    for (idx, module) in chunks.modules.iter_mut().enumerate() {
        tracing::info!(
            "Writing module {} to {}",
            idx,
            args.out_dir
                .join(format!(
                    "module_{}_{}.wasm",
                    idx,
                    module.component_name.as_ref().unwrap()
                ))
                .display()
        );
        std::fs::write(
            args.out_dir.join(format!(
                "module_{}_{}.wasm",
                idx,
                module.component_name.as_ref().unwrap()
            )),
            &module.bytes,
        )
        .expect("failed to write chunk");
    }
}

fn emit_js(chunks: &[SplitModule], modules: &[SplitModule]) -> String {
    use std::fmt::Write;
    let mut glue = format!(
        r#"import {{ initSync }} from "./main.js";
{}"#,
        include_str!("./__wasm_split.js")
    );

    for (idx, chunk) in chunks.iter().enumerate() {
        tracing::debug!("emitting chunk: {:?}", chunk.module_name);
        writeln!(
                glue,
                "export const __wasm_split_load_chunk_{idx} = makeLoad(\"/harness/split/chunk_{idx}_{module}.wasm\", [], fusedImports, initSync);",
                module = chunk.module_name
            ).expect("failed to write to string");
    }

    // Now write the modules
    for (idx, module) in modules.iter().enumerate() {
        let deps = module
            .relies_on_chunks
            .iter()
            .map(|idx| format!("__wasm_split_load_chunk_{idx}"))
            .collect::<Vec<_>>()
            .join(", ");
        let hash_id = module.hash_id.as_ref().unwrap();

        writeln!(
                glue,
                "export const __wasm_split_load_{module}_{hash_id}_{cname} = makeLoad(\"/harness/split/module_{idx}_{cname}.wasm\", [{deps}], fusedImports, initSync);",
                module = module.module_name,
                idx = idx,
                cname = module.component_name.as_ref().unwrap(),
                deps = deps
            )
            .expect("failed to write to string");
    }

    glue
}

#[derive(Parser)]
struct ValidateArgs {
    /// The input wasm file to validate
    main: PathBuf,

    chunks: Vec<PathBuf>,
}

fn validate(args: ValidateArgs) {
    let bytes = std::fs::read(&args.main).expect("failed to read input file");
    let main_module = walrus::Module::from_buffer(&bytes).unwrap();

    for chunk in args.chunks {
        let bytes = std::fs::read(chunk).expect("failed to read input file");
        let chunk_module = walrus::Module::from_buffer(&bytes).unwrap();

        assert!(chunk_module.tables.iter().count() == 1);

        for import in chunk_module.imports.iter() {
            let matching = main_module.exports.iter().find(|e| e.name == import.name);

            let Some(matching) = matching else {
                tracing::error!("Could not find matching export for import {import:#?}");
                continue;
            };

            tracing::debug!("import: {:?}", matching.name);
        }
    }
}
