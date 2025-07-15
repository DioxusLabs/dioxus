use super::*;
use crate::{Result, StructuredOutput};
use anyhow::bail;
use dioxus_rsx::{BodyNode, CallBody, TemplateBody};

/// Translate some source file into Dioxus code
#[derive(Clone, Debug, Parser)]
pub(crate) struct Translate {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[clap(short, long)]
    pub(crate) component: bool,

    /// Input file
    #[clap(short, long)]
    pub(crate) file: Option<String>,

    /// Input file
    #[clap(short, long)]
    pub(crate) raw: Option<String>,

    /// Output file, stdout if not present
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
}

impl Translate {
    pub(crate) fn translate(self) -> Result<StructuredOutput> {
        // Get the right input for the translation
        let contents = determine_input(self.file, self.raw)?;

        // Ensure we're loading valid HTML
        let dom = html_parser::Dom::parse(&contents)?;

        // Convert the HTML to RSX
        let html = convert_html_to_formatted_rsx(&dom, self.component);

        // Write the output
        // todo(jon): we should probably use tracing out a different output format
        // right now we're just printing to stdout since some tools rely on that, but likely we don't want that
        // instead we should be printing as json (or maybe even a different format) if we're not interactive
        match self.output {
            Some(output) => std::fs::write(output, &html)?,
            None => print!("{html}"),
        }

        Ok(StructuredOutput::HtmlTranslate { html })
    }
}

pub fn convert_html_to_formatted_rsx(dom: &Dom, component: bool) -> String {
    let callbody = dioxus_rsx_rosetta::rsx_from_html(dom);

    match component {
        true => write_callbody_with_icon_section(callbody),
        false => dioxus_autofmt::write_block_out(&callbody).unwrap(),
    }
}

fn write_callbody_with_icon_section(mut callbody: CallBody) -> String {
    let mut svgs = vec![];

    dioxus_rsx_rosetta::collect_svgs(&mut callbody.body.roots, &mut svgs);

    let mut out = write_component_body(dioxus_autofmt::write_block_out(&callbody).unwrap());

    if !svgs.is_empty() {
        write_svg_section(&mut out, svgs);
    }

    out
}

fn write_component_body(raw: String) -> String {
    let mut out = String::from("fn component() -> Element {\n    rsx! {");
    indent_and_write(&raw, 1, &mut out);
    out.push_str("    })\n}");
    out
}

fn write_svg_section(out: &mut String, svgs: Vec<BodyNode>) {
    out.push_str("\n\nmod icons {");
    out.push_str("\n    use super::*;");
    for (idx, icon) in svgs.into_iter().enumerate() {
        let raw =
            dioxus_autofmt::write_block_out(&CallBody::new(TemplateBody::new(vec![icon]))).unwrap();
        out.push_str("\n\n    pub(crate) fn icon_");
        out.push_str(&idx.to_string());
        out.push_str("() -> Element {\n        rsx! {");
        indent_and_write(&raw, 2, out);
        out.push_str("        })\n    }");
    }

    out.push_str("\n}");
}

fn indent_and_write(raw: &str, idx: usize, out: &mut String) {
    for line in raw.lines() {
        for _ in 0..idx {
            out.push_str("    ");
        }
        out.push_str(line);
        out.push('\n');
    }
}

fn determine_input(file: Option<String>, raw: Option<String>) -> Result<String> {
    use std::io::IsTerminal as _;

    // Make sure not both are specified
    if file.is_some() && raw.is_some() {
        bail!("Only one of --file or --raw should be specified.");
    }

    if let Some(raw) = raw {
        return Ok(raw);
    }

    if let Some(file) = file {
        return Ok(std::fs::read_to_string(file)?);
    }

    // If neither exist, we try to read from stdin
    if std::io::stdin().is_terminal() {
        bail!("No input file, source, or stdin to translate from.");
    }

    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    Ok(buffer.trim().to_string())
}
