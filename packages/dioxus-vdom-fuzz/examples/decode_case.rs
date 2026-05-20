use dioxus_vdom_fuzz::decode_case;
use std::env;

fn main() {
    let path = env::args().nth(1).expect("usage: decode_case <artifact>");
    let bytes = std::fs::read(&path).expect("read artifact");
    let case = decode_case(&bytes).expect("decode");
    println!("{:#?}", case);
}
