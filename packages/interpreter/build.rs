use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=interpreter.ts");

    match Command::new("tsc").spawn() {
        Ok(_) => println!("Was spawned :)"),
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                println!("`tsc` was not found! Not going to generate new interpreter")
            } else {
                println!("Some strange error occurred :(");
            }
        }
    }
}
