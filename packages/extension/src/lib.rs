//! This file exports functions into the vscode extension

use dioxus_autofmt::{FormattedBlock, IndentOptions, IndentType};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn format_rsx(raw: String, use_tabs: bool, indent_size: usize) -> String {
    let block = dioxus_autofmt::fmt_block(
        &raw,
        0,
        IndentOptions::new(
            if use_tabs {
                IndentType::Tabs
            } else {
                IndentType::Spaces
            },
            indent_size,
            false,
        ),
    );
    block.unwrap()
}

#[wasm_bindgen]
pub fn format_selection(raw: String, use_tabs: bool, indent_size: usize) -> String {
    let block = dioxus_autofmt::fmt_block(
        &raw,
        0,
        IndentOptions::new(
            if use_tabs {
                IndentType::Tabs
            } else {
                IndentType::Spaces
            },
            indent_size,
            false,
        ),
    );
    block.unwrap()
}

#[wasm_bindgen]
pub struct FormatBlockInstance {
    new: String,
    _edits: Vec<FormattedBlock>,
}

#[wasm_bindgen]
impl FormatBlockInstance {
    #[wasm_bindgen]
    pub fn formatted(&self) -> String {
        self.new.clone()
    }

    #[wasm_bindgen]
    pub fn length(&self) -> usize {
        self._edits.len()
    }
}

#[wasm_bindgen]
pub fn format_file(contents: String, use_tabs: bool, indent_size: usize) -> FormatBlockInstance {
    let _edits = dioxus_autofmt::fmt_file(
        &contents,
        IndentOptions::new(
            if use_tabs {
                IndentType::Tabs
            } else {
                IndentType::Spaces
            },
            indent_size,
            false,
        ),
    );
    let out = dioxus_autofmt::apply_formats(&contents, _edits.clone());
    FormatBlockInstance { new: out, _edits }
}

#[wasm_bindgen]
pub fn translate_rsx(contents: String, _component: bool) -> String {
    // Ensure we're loading valid HTML
    let dom = html_parser::Dom::parse(&contents).unwrap();

    let callbody = rsx_rosetta::rsx_from_html(&dom);

    // Convert the HTML to RSX
    dioxus_autofmt::write_block_out(callbody).unwrap()
}
