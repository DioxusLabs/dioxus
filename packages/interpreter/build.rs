use std::collections::hash_map::DefaultHasher;
use std::env;
use std::path::PathBuf;
use std::{hash::Hasher, process::Command};

fn main() {
    // If any TS changes, re-run the build script
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ts_path = manifest_dir.clone();
    ts_path.push("src/ts");
    let watching = std::fs::read_dir(ts_path).unwrap();
    let ts_paths: Vec<_> = watching
        .into_iter()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    for path in &ts_paths {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    // Compute the hash of the ts files
    let hash = hash_ts_files(ts_paths);

    // If the hash matches the one on disk, we're good and don't need to update bindings
    let hash_file = manifest_dir.join("src/js/hash.txt");
    let fs_hash_string = std::fs::read_to_string(&hash_file);
    let expected = fs_hash_string
        .as_ref()
        .map(|s| s.trim())
        .unwrap_or_default();
    if expected == hash.to_string() {
        return;
    }

    // Otherwise, generate the bindings and write the new hash to disk
    // Generate the bindings for both native and web
    gen_bindings("common", "common");
    gen_bindings("native", "native");
    gen_bindings("core", "core");
    gen_bindings("eval", "eval");
    gen_bindings("native_eval", "native_eval");
    gen_bindings("hydrate", "hydrate");
    gen_bindings("initialize_streaming", "initialize_streaming");

    std::fs::write(hash_file, hash.to_string()).unwrap();
}

/// Hashes the contents of a directory
fn hash_ts_files(files: Vec<PathBuf>) -> u64 {
    let mut hash = DefaultHasher::new();
    for file in files {
        let contents = std::fs::read_to_string(file).unwrap();
        // windows + git does a weird thing with line endings, so we need to normalize them
        for line in contents.lines() {
            hash.write(line.trim_matches('\r').as_bytes());
        }
    }
    hash.finish()
}

// okay...... so tsc might fail if the user doesn't have it installed
// we don't really want to fail if that's the case
// but if you started *editing* the .ts files, you're gonna have a bad time
// so.....
// we need to hash each of the .ts files and add that hash to the JS files
// if the hashes don't match, we need to fail the build
// that way we also don't need
fn gen_bindings(input_name: &str, output_name: &str) {
    // If the file is generated, and the hash is different, we need to generate it
    let status = Command::new("bun")
        .arg("build")
        .arg(format!("src/ts/{input_name}.ts"))
        .arg("--outfile")
        .arg(format!("src/js/{output_name}.js"))
        .arg("--minify-whitespace")
        .arg("--minify-syntax")
        .status()
        .unwrap();

    if !status.success() {
        panic!(
            "Failed to generate bindings for {}. Make sure you have tsc installed",
            input_name
        );
    }
}
