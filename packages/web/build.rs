use std::{fs::File, io::Write};

fn main() {
    // write the interpreter code to a local file
    let mut file = File::create("interpreter.js").unwrap();
    file.write_all(dioxus_interpreter_js::INTERPRTER_JS.as_bytes())
        .unwrap();
}
