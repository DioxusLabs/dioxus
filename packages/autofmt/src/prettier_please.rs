use dioxus_rsx::CallBody;
use syn::{parse::Parser, visit_mut::VisitMut, Expr, File, Item, MacroDelimiter};

use crate::{IndentOptions, Writer};

impl Writer<'_> {
    pub fn unparse_expr(&mut self, expr: &Expr) -> String {
        unparse_expr(expr, self.raw_src, &self.out.indent)
    }
}

// we use weird unicode alternatives to avoid conflicts with the actual rsx! macro
const MARKER: &str = "𝕣𝕤𝕩";
const MARKER_REPLACE: &str = "𝕣𝕤𝕩! {}";

pub fn unparse_expr(expr: &Expr, src: &str, cfg: &IndentOptions) -> String {
    struct ReplaceMacros<'a> {
        src: &'a str,
        formatted_stack: Vec<String>,
        cfg: &'a IndentOptions,
    }

    impl VisitMut for ReplaceMacros<'_> {
        fn visit_macro_mut(&mut self, i: &mut syn::Macro) {
            // replace the macro with a block that roughly matches the macro
            if let Some("rsx" | "render") = i
                .path
                .segments
                .last()
                .map(|i| i.ident.to_string())
                .as_deref()
            {
                // format the macro in place
                // we'll use information about the macro to replace it with another formatted block
                // once we've written out the unparsed expr from prettyplease, we can replace
                // this dummy block with the actual formatted block
                let body = CallBody::parse_strict.parse2(i.tokens.clone()).unwrap();
                let multiline = !Writer::is_short_rsx_call(&body.body.roots);
                let mut formatted = {
                    let mut writer = Writer::new(self.src, self.cfg.clone());
                    _ = writer.write_body_nodes(&body.body.roots).ok();
                    writer.consume()
                }
                .unwrap();

                i.path = syn::parse_str(MARKER).unwrap();
                i.tokens = Default::default();

                // make sure to transform the delimiter to a brace so the marker can be found
                // an alternative approach would be to use multiple different markers that are not
                // sensitive to the delimiter.
                i.delimiter = MacroDelimiter::Brace(Default::default());

                // Push out the indent level of the formatted block if it's multiline
                if multiline || formatted.contains('\n') {
                    formatted = formatted
                        .lines()
                        .map(|line| {
                            // Don't add indentation to blank lines (avoid trailing whitespace)
                            if line.is_empty() {
                                String::new()
                            } else {
                                format!("{}{line}", self.cfg.indent_str())
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                }

                // Save this formatted block for later, when we apply it to the original expr
                self.formatted_stack.push(formatted)
            }

            syn::visit_mut::visit_macro_mut(self, i);
        }
    }

    // Visit the expr and replace the macros with formatted blocks
    let mut replacer = ReplaceMacros {
        src,
        cfg,
        formatted_stack: vec![],
    };

    // builds the expression stack
    let mut modified_expr = expr.clone();
    replacer.visit_expr_mut(&mut modified_expr);

    // now unparsed with the modified expression
    let mut unparsed = unparse_inner(&modified_expr);

    // now we can replace the macros with the formatted blocks
    for fmted in replacer.formatted_stack.drain(..) {
        let is_multiline = fmted.ends_with('}') || fmted.contains('\n');
        let is_empty = fmted.trim().is_empty();

        let mut out_fmt = String::from("rsx! {");
        if is_multiline {
            out_fmt.push('\n');
        } else if !is_empty {
            out_fmt.push(' ');
        }

        let mut whitespace = 0;

        for line in unparsed.lines() {
            if line.contains(MARKER) {
                whitespace = line.matches(cfg.indent_str()).count();
                break;
            }
        }

        let mut lines = fmted.lines().enumerate().peekable();

        while let Some((_idx, fmt_line)) = lines.next() {
            // Push the indentation (but not for blank lines - avoid trailing whitespace)
            if is_multiline && !fmt_line.is_empty() {
                out_fmt.push_str(&cfg.indent_str().repeat(whitespace));
            }

            // Calculate delta between indentations - the block indentation is too much
            out_fmt.push_str(fmt_line);

            // Push a newline if there's another line
            if lines.peek().is_some() {
                out_fmt.push('\n');
            }
        }

        if is_multiline {
            out_fmt.push('\n');
            out_fmt.push_str(&cfg.indent_str().repeat(whitespace));
        } else if !is_empty {
            out_fmt.push(' ');
        }

        // Replace the dioxus_autofmt_block__________ token with the formatted block
        out_fmt.push('}');

        unparsed = unparsed.replacen(MARKER_REPLACE, &out_fmt, 1);
        continue;
    }

    // stylistic choice to trim whitespace around the expr
    if unparsed.starts_with("{ ") && unparsed.ends_with(" }") {
        let mut out_fmt = String::new();
        out_fmt.push('{');
        out_fmt.push_str(&unparsed[2..unparsed.len() - 2]);
        out_fmt.push('}');
        out_fmt
    } else {
        unparsed
    }
}

/// Unparse an expression back into a string
///
/// This creates a new temporary file, parses the expression into it, and then formats the file.
/// This is a bit of a hack, but dtonlay doesn't want to support this very simple usecase, forcing us to clone the expr
pub fn unparse_inner(expr: &Expr) -> String {
    let file = wrapped(expr);
    let wrapped = prettyplease::unparse(&file);
    unwrapped(wrapped)
}

// Split off the fn main and then cut the tabs off the front
fn unwrapped(raw: String) -> String {
    let mut o = raw
        .strip_prefix("fn main() {\n")
        .unwrap()
        .strip_suffix("}\n")
        .unwrap()
        .lines()
        .map(|line| line.strip_prefix("    ").unwrap_or_default()) // todo: set this to tab level
        .collect::<Vec<_>>()
        .join("\n");

    // remove the semicolon
    if o.ends_with(';') {
        o.pop();
    }

    o
}

fn wrapped(expr: &Expr) -> File {
    File {
        shebang: None,
        attrs: vec![],
        items: vec![
            //
            Item::Verbatim(quote::quote! {
                fn main() {
                    #expr;
                }
            }),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;

    fn fmt_block_from_expr(raw: &str, tokens: TokenStream, cfg: IndentOptions) -> Option<String> {
        let body = CallBody::parse_strict.parse2(tokens).unwrap();
        let mut writer = Writer::new(raw, cfg);
        writer.write_body_nodes(&body.body.roots).ok()?;
        writer.consume()
    }

    #[test]
    fn unparses_raw() {
        let expr = syn::parse_str("1 + 1").expect("Failed to parse");
        let unparsed = prettyplease::unparse(&wrapped(&expr));
        assert_eq!(unparsed, "fn main() {\n    1 + 1;\n}\n");
    }

    #[test]
    fn weird_ifcase() {
        let contents = r##"
        fn main() {
            move |_| timer.with_mut(|t| if t.started_at.is_none() { Some(Instant::now()) } else { None })
        }
    "##;

        let expr: File = syn::parse_file(contents).unwrap();
        let out = prettyplease::unparse(&expr);
        println!("{}", out);
    }

    #[test]
    fn multiline_madness() {
        let contents = r##"
        {
        {children.is_some().then(|| rsx! {
            span {
                class: "inline-block ml-auto hover:bg-gray-500",
                onclick: move |evt| {
                    evt.cancel_bubble();
                },
                icons::icon_5 {}
                {rsx! {
                    icons::icon_6 {}
                }}
            }
        })}
        {children.is_some().then(|| rsx! {
            span {
                class: "inline-block ml-auto hover:bg-gray-500",
                onclick: move |evt| {
                    evt.cancel_bubble();
                },
                icons::icon_10 {}
            }
        })}

        }

        "##;

        let expr: Expr = syn::parse_str(contents).unwrap();
        let out = unparse_expr(&expr, contents, &IndentOptions::default());
        println!("{}", out);
    }

    #[test]
    fn write_body_no_indent() {
        let src = r##"
            span {
                class: "inline-block ml-auto hover:bg-gray-500",
                onclick: move |evt| {
                    evt.cancel_bubble();
                },
                icons::icon_10 {}
                icons::icon_10 {}
                icons::icon_10 {}
                icons::icon_10 {}
                div { "hi" }
                div { div {} }
                div { div {} div {} div {} }
                {children}
                {
                    some_big_long()
                        .some_big_long()
                        .some_big_long()
                        .some_big_long()
                        .some_big_long()
                        .some_big_long()
                }
                div { class: "px-4", {is_current.then(|| rsx! { {children} })} }
                Thing {
                    field: rsx! {
                        div { "hi" }
                        Component {
                            onrender: rsx! {
                                div { "hi" }
                                Component {
                                    onclick: move |_| {
                                        another_macro! {
                                            div { class: "max-w-lg lg:max-w-2xl mx-auto mb-16 text-center",
                                                "gomg"
                                                "hi!!"
                                                "womh"
                                            }
                                        };
                                        rsx! {
                                            div { class: "max-w-lg lg:max-w-2xl mx-auto mb-16 text-center",
                                                "gomg"
                                                "hi!!"
                                                "womh"
                                            }
                                        };
                                        println!("hi")
                                    },
                                    onrender: move |_| {
                                        let _ = 12;
                                        let r = rsx! {
                                            div { "hi" }
                                        };
                                        rsx! {
                                            div { "hi" }
                                        }
                                    }
                                }
                                {
                                    rsx! {
                                        BarChart {
                                            id: "bar-plot".to_string(),
                                            x: value,
                                            y: label
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "##;

        let tokens: TokenStream = syn::parse_str(src).unwrap();
        let out = fmt_block_from_expr(src, tokens, IndentOptions::default()).unwrap();
        println!("{}", out);
    }

    #[test]
    fn write_component_body() {
        let src = r##"
    div { class: "px-4", {is_current.then(|| rsx! { {children} })} }
    "##;

        let tokens: TokenStream = syn::parse_str(src).unwrap();
        let out = fmt_block_from_expr(src, tokens, IndentOptions::default()).unwrap();
        println!("{}", out);
    }

    #[test]
    fn weird_macro() {
        let contents = r##"
        fn main() {
            move |_| {
                drop_macro_semi! {
                    "something_very_long_something_very_long_something_very_long_something_very_long"
                };
                let _ = drop_macro_semi! {
                    "something_very_long_something_very_long_something_very_long_something_very_long"
                };
                drop_macro_semi! {
                    "something_very_long_something_very_long_something_very_long_something_very_long"
                };
            };
        }
    "##;

        let expr: File = syn::parse_file(contents).unwrap();
        let out = prettyplease::unparse(&expr);
        println!("{}", out);
    }

    #[test]
    fn comments_on_nodes() {
        let src = r##"// hiasdasds
    div {
        attr: "value", // comment
        div {}
        "hi" // hello!
        "hi" // hello!
        "hi" // hello!
        // hi!
    }
    "##;

        let tokens: TokenStream = syn::parse_str(src).unwrap();
        let out = fmt_block_from_expr(src, tokens, IndentOptions::default()).unwrap();
        println!("{}", out);
    }
}
