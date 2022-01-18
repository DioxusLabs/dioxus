use html_parser::Dom;
use html_parser::Node;
use std::fmt::Write;
use std::io::Read;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;

pub mod extract_svgs;
pub mod to_component;
pub mod translate;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "translate")]
pub struct Translate {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    pub component: bool,

    /// Input file
    #[structopt(short, long)]
    pub source: Option<String>,

    /// Input file
    #[structopt(short, long, parse(from_os_str))]
    pub input: Option<PathBuf>,

    /// Output file, stdout if not present
    #[structopt(parse(from_os_str))]
    pub output: Option<PathBuf>,
}

impl Translate {
    pub fn translate(self) -> anyhow::Result<()> {
        let Translate {
            component: as_component,
            input,
            output,
            source,
        } = self;

        let contents;
        let temp = input.map(|f| {
            std::fs::read_to_string(&f).unwrap_or_else(|e| {
                log::error!("Cloud not read input file: {}.", e);
                exit(0);
            })
        });
        if let None = temp {
            if let Some(s) = source {
                contents = s;
            } else {
                if atty::is(atty::Stream::Stdin) {
                    log::error!("No input file, source, or stdin to translate from.");
                    exit(0);
                }

                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer).unwrap();

                contents = buffer.trim().to_string();
            }
        } else {
            contents = temp.unwrap();
        }

        // parse the input as html and prepare the output
        let dom = Dom::parse(&contents)?;
        let mut out_buf = String::new();

        if as_component {
            writeln!(out_buf, "use dioxus::prelude::*;")?;
            writeln!(out_buf, "fn component(cx: Scope) -> Element {{")?;
            writeln!(out_buf, "    cx.render(rsx!(")?;
        }
        for child in &dom.children {
            render_child(&mut out_buf, child, if as_component { 2 } else { 0 })?;
        }
        if as_component {
            writeln!(out_buf, "    ))")?;
            writeln!(out_buf, "}}")?;
        }

        if let Some(output) = output {
            std::fs::write(&output, out_buf)?;
        } else {
            print!("{}", out_buf);
        }

        Ok(())
    }
}

fn render_child(f: &mut impl Write, child: &Node, il: u32) -> std::fmt::Result {
    write_tabs(f, il)?;
    match child {
        Node::Text(t) => writeln!(f, "\"{}\"", t)?,
        Node::Comment(e) => writeln!(f, "/* {} */", e)?,
        Node::Element(el) => {
            // open the tag
            write!(f, "{} {{ ", &el.name)?;

            // todo: dioxus will eventually support classnames
            // for now, just write them with a space between each
            let class_iter = &mut el.classes.iter();
            if let Some(first_class) = class_iter.next() {
                write!(f, "class: \"{}", first_class)?;
                for next_class in class_iter {
                    write!(f, " {}", next_class)?;
                }
                write!(f, "\",")?;
            }
            write!(f, "\n")?;

            // write the attributes
            if let Some(id) = &el.id {
                write_tabs(f, il + 1)?;
                writeln!(f, "id: \"{}\",", id)?;
            }

            for (name, value) in &el.attributes {
                write_tabs(f, il + 1)?;

                use convert_case::{Case, Casing};
                if name.chars().any(|ch| ch.is_ascii_uppercase() || ch == '-') {
                    let new_name = name.to_case(Case::Snake);
                    match value {
                        Some(val) => writeln!(f, "{}: \"{}\",", new_name, val)?,
                        None => writeln!(f, "{}: \"\",", new_name)?,
                    }
                } else {
                    match name.as_str() {
                        "for" | "async" | "type" | "as" => write!(f, "r#")?,
                        _ => {}
                    }

                    match value {
                        Some(val) => writeln!(f, "{}: \"{}\",", name, val)?,
                        None => writeln!(f, "{}: \"\",", name)?,
                    }
                }
            }

            // now the children
            for child in &el.children {
                render_child(f, child, il + 1)?;
            }

            // close the tag
            write_tabs(f, il)?;
            writeln!(f, "}}")?;
        }
    };
    Ok(())
}

fn write_tabs(f: &mut impl Write, num: u32) -> std::fmt::Result {
    for _ in 0..num {
        write!(f, "    ")?
    }
    Ok(())
}
