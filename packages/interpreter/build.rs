use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=interpreter.ts");
    Command::new("tsc").spawn().unwrap();
}
