use std::{
    collections::hash_map::DefaultHasher, fs::read_to_string, hash::Hasher, process::Command,
};

fn main() {
    // If any TS changes, re-run the build script
    println!("cargo:rerun-if-changed=src/ts/*.ts");

    // Compute the hash of the ts files
    let hash = hash_dir("./src/ts");

    // If the hash matches the one on disk, we're good and don't need to update bindings
    if let Ok(contents) = read_to_string("./src/js/hash.txt") {
        if contents.trim() == hash.to_string() {
            return;
        }
    }

    // Otherwise, generate the bindings and write the new hash to disk
    // Generate the bindings for both native and web
    gen_bindings("common", "common");
    gen_bindings("native", "native");
    gen_bindings("core", "core");

    std::fs::write("src/js/hash.txt", hash.to_string()).unwrap();
}

/// Hashes the contents of a directory
fn hash_dir(dir: &str) -> u128 {
    let mut out = 0;

    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        let Some(ext) = path.extension() else {
            continue;
        };

        if ext != "ts" {
            continue;
        }

        // Hash the contents of the file and then add it to the overall hash
        // This makes us order invariant
        let mut hasher = DefaultHasher::new();
        hasher.write(&std::fs::read(&path).unwrap());
        out += hasher.finish() as u128;
    }

    out
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
