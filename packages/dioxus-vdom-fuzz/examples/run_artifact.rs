use dioxus_vdom_fuzz::{decode_case, run_case};
use std::env;

fn main() {
    let path = env::args().nth(1).expect("usage: run_artifact <artifact>");
    let bytes = std::fs::read(&path).expect("read artifact");
    let case = decode_case(&bytes).expect("decode");
    match run_case(&case) {
        Ok(()) => println!("ok"),
        Err(failure) => println!("failure: {failure}"),
    }
}
