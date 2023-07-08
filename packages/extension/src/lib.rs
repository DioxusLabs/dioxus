//! This file exports functions into the vscode extension

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greeet() {
    //
}

#[wasm_bindgen]
pub fn format_rsx(raw: String) -> String {
    let block = dioxus_autofmt::fmt_block(&raw, 0);
    block.unwrap()
}

#[wasm_bindgen]
pub fn translate_rsx(contents: String, component: bool) -> String {
    // Ensure we're loading valid HTML
    let dom = html_parser::Dom::parse(&contents).unwrap();

    let callbody = rsx_rosetta::rsx_from_html(&dom);

    // Convert the HTML to RSX
    let out = dioxus_autofmt::write_block_out(callbody).unwrap();

    out
}

// rsx! {
//     div {}
//     div {}
//     div {}
//     div {} div {}
// }
