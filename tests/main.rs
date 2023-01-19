use std::error::Error;
use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs


#[test]
fn ready() {
    println!("Compiled successfully!")
}

#[test]
fn test_create() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dioxus")?;
    cmd.arg("create").arg("scratch");
    cmd.assert()
        .failure();

    Ok(())
}