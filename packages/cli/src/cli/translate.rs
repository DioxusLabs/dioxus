use std::process::exit;

use dioxus_rsx::{BodyNode, CallBody};

use super::*;

/// Translate some source file into Dioxus code
#[derive(Clone, Debug, Parser)]
#[clap(name = "translate")]
pub struct Translate {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[clap(short, long)]
    pub component: bool,

    /// Input file
    #[clap(short, long)]
    pub file: Option<String>,

    /// Input file
    #[clap(short, long)]
    pub raw: Option<String>,

    /// Output file, stdout if not present
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

impl Translate {
    pub fn translate(self) -> Result<()> {
        // Get the right input for the translation
        let contents = determine_input(self.file, self.raw)?;

        // Ensure we're loading valid HTML
        let dom = html_parser::Dom::parse(&contents)?;

        // Convert the HTML to RSX
        let out = convert_html_to_formatted_rsx(&dom, self.component);

        // Write the output
        match self.output {
            Some(output) => std::fs::write(output, out)?,
            None => print!("{}", out),
        }

        Ok(())
    }
}

pub fn convert_html_to_formatted_rsx(dom: &Dom, component: bool) -> String {
    let callbody = rsx_rosetta::rsx_from_html(dom);

    match component {
        true => write_callbody_with_icon_section(callbody),
        false => dioxus_autofmt::write_block_out(callbody).unwrap(),
    }
}

fn write_callbody_with_icon_section(mut callbody: CallBody) -> String {
    let mut svgs = vec![];

    rsx_rosetta::collect_svgs(&mut callbody.roots, &mut svgs);

    let mut out = write_component_body(dioxus_autofmt::write_block_out(callbody).unwrap());

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
        let raw = dioxus_autofmt::write_block_out(CallBody { roots: vec![icon] }).unwrap();
        out.push_str("\n\n    pub fn icon_");
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
    // Make sure not both are specified
    if file.is_some() && raw.is_some() {
        log::error!("Only one of --file or --raw should be specified.");
        exit(0);
    }

    if let Some(raw) = raw {
        return Ok(raw);
    }

    if let Some(file) = file {
        return Ok(std::fs::read_to_string(file)?);
    }

    // If neither exist, we try to read from stdin
    if atty::is(atty::Stream::Stdin) {
        return custom_error!("No input file, source, or stdin to translate from.");
    }

    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    Ok(buffer.trim().to_string())
}

#[test]
fn generates_svgs() {
    let st = include_str!("../../tests/svg.html");

    let out = convert_html_to_formatted_rsx(&html_parser::Dom::parse(st).unwrap(), true);

    println!("{}", out);
}
