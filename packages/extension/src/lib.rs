use dioxus_autofmt::{FormattedBlock, IndentOptions, IndentType};
use std::{cell::OnceCell, io};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const VSCODE_TYPESCRIPT_IMPORT: &str = "import type * as vscode from 'vscode';";

#[wasm_bindgen]
extern "C" {
    pub type OutputChannel;

    #[wasm_bindgen(method)]
    pub fn appendLine(this: &OutputChannel, value: &str);
}

thread_local! {
    static OUTPUT_CHANNEL: OnceCell<OutputChannel> = const { OnceCell::new() };
}

pub struct OutputChannelWriter;

impl io::Write for OutputChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let msg = String::from_utf8_lossy(buf);

        OUTPUT_CHANNEL.with(|cell| {
            if let Some(channel) = cell.get() {
                channel.appendLine(&msg);
            }
        });

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[wasm_bindgen(js_name = initTracing)]
pub fn init_tracing(
    #[wasm_bindgen(unchecked_param_type = "vscode.OutputChannel")] channel: OutputChannel,
) {
    _ = OUTPUT_CHANNEL.with(|cell| cell.set(channel));

    let layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_writer(|| OutputChannelWriter);

    _ = tracing_subscriber::registry().with(layer).try_init();
}

#[wasm_bindgen(js_name = formatRsx)]
pub fn format_rsx(raw: String, use_tabs: bool, indent_size: usize) -> String {
    dioxus_autofmt::fmt_block(
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
    )
    .unwrap()
}

#[wasm_bindgen(js_name = formatSelection)]
pub fn format_selection(
    raw: String,
    use_tabs: bool,
    indent_size: usize,
    base_indent: usize,
) -> String {
    dioxus_autofmt::fmt_block(
        &raw,
        base_indent,
        IndentOptions::new(
            if use_tabs {
                IndentType::Tabs
            } else {
                IndentType::Spaces
            },
            indent_size,
            false,
        ),
    )
    .unwrap()
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

#[wasm_bindgen(js_name = formatFile)]
pub fn format_file(contents: String, use_tabs: bool, indent_size: usize) -> FormatBlockInstance {
    // TODO: use rustfmt for this instead

    let options = IndentOptions::new(
        if use_tabs {
            IndentType::Tabs
        } else {
            IndentType::Spaces
        },
        indent_size,
        false,
    );

    let edits = syn::parse_file(&contents)
        .map(|file| dioxus_autofmt::try_fmt_file(&contents, &file, options));

    let Ok(Ok(_edits)) = edits else {
        return FormatBlockInstance {
            new: contents,
            _edits: Vec::new(),
        };
    };

    FormatBlockInstance {
        new: dioxus_autofmt::apply_formats(&contents, _edits.clone()),
        _edits,
    }
}

#[wasm_bindgen(js_name = translateRsx)]
pub fn translate_rsx(contents: String, _component: bool) -> String {
    let dom = html_parser::Dom::parse(&contents).unwrap();
    let callbody = dioxus_rsx_rosetta::rsx_from_html(&dom);

    dioxus_autofmt::write_block_out(&callbody).unwrap()
}
