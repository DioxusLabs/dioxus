use std::{
    fs::read_to_string,
    hash::{DefaultHasher, Hash, Hasher},
    process::Command,
};

fn main() {
    // If any TS changes, re-run the build script
    println!("cargo:rerun-if-changed=src/ts/*.ts");
    println!("cargo:rerun-if-changed=*.json");

    // Compute the hash of the ts files
    let hash = hash_dir("src/ts");

    // If the hash matches the one on disk, we're good and don't need to update bindings
    if let Ok(contents) = read_to_string("src/js/hash.txt") {
        if contents.trim() == hash.to_string() {
            return;
        }
    }

    // Otherwise, generate the bindings and write the new hash to disk
    // Generate the bindings for both native and web
    gen_bindings("native");
    gen_bindings("web");

    std::fs::write("src/js/hash.txt", hash.to_string()).unwrap();
}

/// Hashes the contents of a directory
fn hash_dir(dir: &str) -> u64 {
    let mut hasher = DefaultHasher::new();

    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let metadata = std::fs::metadata(&path).unwrap();
        if metadata.is_file() {
            let contents = std::fs::read(&path).unwrap();
            contents.hash(&mut hasher);
        }
    }

    hasher.finish()
}

// okay...... so tsc might fail if the user doesn't have it installed
// we don't really want to fail if that's the case
// but if you started *editing* the .ts files, you're gonna have a bad time
// so.....
// we need to hash each of the .ts files and add that hash to the JS files
// if the hashes don't match, we need to fail the build
// that way we also don't need
fn gen_bindings(name: &str) {
    // If the file is generated, and the hash is different, we need to generate it
    let status = Command::new("tsc")
        .arg("--p")
        .arg(format!("tsconfig.{name}.json"))
        .status()
        .unwrap();

    if !status.success() {
        panic!(
            "Failed to generate bindings for {}. Make sure you have tsc installed",
            name
        );
    }
}
