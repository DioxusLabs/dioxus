use std::process::Command;

fn main() {
    // If any TS changes, re-run the build script
    println!("cargo:rerun-if-changed=src/ts/form.ts");
    println!("cargo:rerun-if-changed=src/ts/core.ts");
    println!("cargo:rerun-if-changed=src/ts/serialize.ts");
    println!("cargo:rerun-if-changed=src/ts/set_attribute.ts");
    println!("cargo:rerun-if-changed=src/ts/common.ts");

    // Compute the hash of the ts files
    let hash = hash_ts_files();

    // If the hash matches the one on disk, we're good and don't need to update bindings
    let expected = include_str!("src/js/hash.txt").trim();
    if expected == hash.to_string() {
        return;
    }

    // Otherwise, generate the bindings and write the new hash to disk
    // Generate the bindings for both native and web
    gen_bindings("common", "common");
    gen_bindings("native", "native");
    gen_bindings("core", "core");

    std::fs::write("src/js/hash.txt", hash.to_string()).unwrap();
}

/// Hashes the contents of a directory
fn hash_ts_files() -> u128 {
    let mut out = 0;

    let files = [
        include_str!("src/ts/common.ts"),
        include_str!("src/ts/native.ts"),
        include_str!("src/ts/core.ts"),
    ];

    // Let's make the dumbest hasher by summing the bytes of the files
    // The location is multiplied by the byte value to make sure that the order of the bytes matters
    let mut idx = 0;
    for file in files {
        // windows + git does a weird thing with line endings, so we need to normalize them
        for line in file.lines() {
            idx += 1;
            for byte in line.bytes() {
                idx += 1;
                out += (byte as u128) * (idx as u128);
            }
        }
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
