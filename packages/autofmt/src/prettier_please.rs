use std::path::Path;

use prettyplease::unparse;
use proc_macro2::TokenStream;
use syn::{visit_mut::VisitMut, Expr, File, Item};

use crate::Writer;

impl Writer<'_> {
    pub fn unparse_expr(&mut self, expr: &Expr) -> String {
        unparse_expr(expr, self.raw_src)
    }
}

const MARKER: &str = "dioxus_autofmt_block__________dioxus_autofmt_block__________";
const MARKER_REPLACE: &str = "dioxus_autofmt_block__________dioxus_autofmt_block__________! {}";

pub fn unparse_expr(expr: &Expr, src: &str) -> String {
    struct ReplaceMacros<'a> {
        src: &'a str,
        formatted_stack: Vec<String>,
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
                let formatted = crate::fmt_block_from_expr(self.src, i.tokens.clone()).unwrap();

                // always push out the rsx to require a new line
                i.path = syn::parse_str(MARKER).unwrap();
                i.tokens = Default::default();
                // i

                // *_expr = syn::Stmt::Expr(
                //     syn::parse_quote!(dioxus_autofmt_block__________),
                //     i.semi_token,
                // );

                // Save this formatted block for later, when we apply it to the original expr
                self.formatted_stack.push(formatted)
            }
            syn::visit_mut::visit_macro_mut(self, i);
        }
    }

    // Visit the expr and replace the macros with formatted blocks
    let mut replacer = ReplaceMacros {
        src: src,
        formatted_stack: vec![],
    };

    // builds the expression stack
    let mut modified_expr = expr.clone();
    syn::visit_mut::visit_expr_mut(&mut replacer, &mut modified_expr);
    // replacer.visit_expr_mut(&mut modified_expr);

    // now unparsed with the modified expression
    let mut unparsed = unparse_inner(&modified_expr);

    // walk each line looking for the dioxus_autofmt_block__________ token
    // if we find it, replace it with the formatted block
    // if there's indentation we want to presreve it

    // now we can replace the macros with the formatted blocks
    for formatted in replacer.formatted_stack.drain(..) {
        // dbg!(&formatted);
        // let fmted = if formatted.contains('\n') {
        //     format!("rsx! {{{formatted}\n}}")
        // } else {
        //     format!("rsx! {{{formatted}}}")
        // };
        let fmted = formatted.trim_start();
        let mut out_fmt = String::from("rsx! ");
        let mut whitespace = 0;

        for line in unparsed.lines() {
            if line.contains(MARKER) {
                whitespace = line.chars().take_while(|c| c.is_whitespace()).count();
                break;
            }
        }

        let mut lines = fmted.lines().enumerate().peekable();

        while let Some((idx, fmt_line)) = lines.next() {
            // Push the indentation
            if idx > 0 {
                out_fmt.push_str(&" ".repeat(whitespace));
            }

            // Calculate delta between indentations - the block indentation is too much
            out_fmt.push_str(fmt_line);

            // Push a newline
            out_fmt.push('\n');
        }

        // Remove the last newline
        out_fmt.pop();

        // Replace the dioxus_autofmt_block__________ token with the formatted block
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
    let wrapped = unparse(&file);
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
        .map(|line| line.strip_prefix("    ").unwrap()) // todo: set this to tab level
        .collect::<Vec<_>>()
        .join("\n");

    // remove the semicolon
    o.pop();

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

#[test]
fn unparses_raw() {
    let expr = syn::parse_str("1 + 1").expect("Failed to parse");
    let unparsed = unparse(&wrapped(&expr));
    assert_eq!(unparsed, "fn main() {\n    1 + 1;\n}\n");
}

// #[test]
// fn unparses_completely() {
//     let expr = syn::parse_str("1 + 1").expect("Failed to parse");
//     let unparsed = unparse_expr(&expr);
//     assert_eq!(unparsed, "1 + 1");
// }

// #[test]
// fn unparses_let_guard() {
//     let expr = syn::parse_str("let Some(url) = &link.location").expect("Failed to parse");
//     let unparsed = unparse_expr(&expr);
//     assert_eq!(unparsed, "let Some(url) = &link.location");
// }

#[test]
fn weird_ifcase() {
    let contents = r##"
    fn main() {
        move |_| timer.with_mut(|t| if t.started_at.is_none() { Some(Instant::now()) } else { None })
    }
"##;

    let expr: File = syn::parse_file(contents).unwrap();
    let out = unparse(&expr);
    println!("{}", out);
}

#[test]
fn multiline_maddness() {
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
    let out = unparse_expr(&expr, &contents);
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
        }
    "##;

    let tokens: TokenStream = syn::parse_str(src).unwrap();
    let out = crate::fmt_block_from_expr(src, tokens).unwrap();
    println!("{}", out);
}
