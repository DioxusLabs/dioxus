use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=DIOXUS_CLI_GIT_SHA");
    println!("cargo:rerun-if-env-changed=DIOXUS_CLI_GIT_SHA_SHORT");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let full_hash = std::env::var("DIOXUS_CLI_GIT_SHA").ok().or_else(|| {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let hash = String::from_utf8(output.stdout).ok()?;
        let hash = hash.trim().to_string();

        if hash.is_empty() {
            None
        } else {
            Some(hash)
        }
    });

    if let Some(full_hash) = full_hash {
        println!("cargo:rustc-env=DIOXUS_CLI_GIT_SHA={full_hash}");

        let short_hash = std::env::var("DIOXUS_CLI_GIT_SHA_SHORT")
            .ok()
            .unwrap_or_else(|| full_hash.chars().take(7).collect());

        println!("cargo:rustc-env=DIOXUS_CLI_GIT_SHA_SHORT={short_hash}");
    }
}
