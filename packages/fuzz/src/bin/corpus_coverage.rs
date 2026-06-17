//! Replay the libFuzzer corpus under normal Rust coverage instrumentation.
//!
//! `cargo fuzz coverage` reports sanitizer edge/features coverage. This binary
//! reuses the same encoded corpus and warmup path to produce source-level
//! coverage with `cargo llvm-cov`.

use dioxus_vdom_fuzz::{
    decode_case, format_failure_report, run_case, warmup_deferred_priority_paths,
};
use std::{env, fs, path::PathBuf, process};

fn main() {
    warmup_deferred_priority_paths();

    let roots = corpus_roots();
    let mut files = Vec::new();
    for root in &roots {
        collect_files(root, &mut files);
    }
    files.sort();

    let mut decoded = 0usize;
    let mut failed = 0usize;
    for file in &files {
        let data = fs::read(file)
            .unwrap_or_else(|err| panic!("failed to read corpus input {}: {err}", file.display()));
        let Some(case) = decode_case(&data) else {
            continue;
        };
        decoded += 1;
        if let Err(failure) = run_case(&case) {
            failed += 1;
            eprintln!(
                "corpus input {} failed:\n{}",
                file.display(),
                format_failure_report(&case, &failure)
            );
        }
    }

    println!(
        "replayed {decoded} decoded corpus inputs from {} files",
        files.len()
    );

    if failed > 0 {
        eprintln!("{failed} corpus inputs failed");
        process::exit(1);
    }
}

fn corpus_roots() -> Vec<PathBuf> {
    let mut args = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if args.is_empty() {
        args.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/corpus/vdom_ops"));
    }
    args
}

fn collect_files(root: &PathBuf, files: &mut Vec<PathBuf>) {
    if root.is_file() {
        files.push(root.clone());
        return;
    }

    for entry in fs::read_dir(root)
        .unwrap_or_else(|err| panic!("failed to read corpus directory {}: {err}", root.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", root.display()))
            .path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}
