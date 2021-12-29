use std::fmt::{Debug, Display, Formatter};

use anyhow::Result;

use html_parser::{Dom, Node};

pub fn translate_from_html_file(target: &str) -> Result<RsxRenderer> {
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(target).unwrap();

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read your file.");
    translate_from_html_to_rsx(&contents, true)
}

pub fn translate_from_html_to_rsx(html: &str, as_call: bool) -> Result<RsxRenderer> {
    let contents = Dom::parse(html)?;
    let renderer = RsxRenderer {
        as_call,
        dom: contents,
    };
    Ok(renderer)
}

pub struct RsxRenderer {
    dom: Dom,
    as_call: bool,
}

impl Display for RsxRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.as_call {
            writeln!(f, r##"use dioxus::prelude::*;"##)?;
            writeln!(f, r##"const Component: FC<()> = |cx| cx.render(rsx!{{"##)?;
        }
        for child in &self.dom.children {
            render_child(f, child, 1)?;
        }
        if self.as_call {
            write!(f, r##"}});"##)?;
        }
        Ok(())
    }
}

fn render_child(f: &mut Formatter<'_>, child: &Node, il: u32) -> std::fmt::Result {
    write_tabs(f, il);
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
                match value {
                    Some(val) => writeln!(f, "{}: \"{}\",", name, val)?,
                    None => writeln!(f, "{}: \"\",", name)?,
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

fn write_tabs(f: &mut Formatter, num: u32) -> std::fmt::Result {
    for _ in 0..num {
        write!(f, "    ")?
    }
    Ok(())
}
