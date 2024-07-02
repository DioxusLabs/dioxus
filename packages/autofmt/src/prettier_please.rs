use prettyplease::unparse;
use syn::{visit_mut::VisitMut, Expr, File, Item};

use crate::Writer;

impl Writer<'_> {
    pub fn unparse_expr(&mut self, expr: &Expr) -> String {
        struct ReplaceMacros<'a, 'b> {
            writer: &'a mut Writer<'b>,
            formatted_stack: Vec<String>,
        }

        impl VisitMut for ReplaceMacros<'_, '_> {
            fn visit_stmt_mut(&mut self, _expr: &mut syn::Stmt) {
                if let syn::Stmt::Macro(i) = _expr {
                    // replace the macro with a block that roughly matches the macro
                    if let Some("rsx" | "render") = i
                        .mac
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
                        let formatted = crate::fmt_block_from_expr(
                            self.writer.raw_src,
                            syn::ExprMacro {
                                attrs: i.attrs.clone(),
                                mac: i.mac.clone(),
                            },
                        )
                        .unwrap();

                        *_expr = syn::Stmt::Expr(
                            syn::parse_quote!(dioxus_autofmt_block__________),
                            i.semi_token,
                        );

                        // Save this formatted block for later, when we apply it to the original expr
                        self.formatted_stack.push(formatted);
                    }
                }

                syn::visit_mut::visit_stmt_mut(self, _expr);
            }

            fn visit_expr_mut(&mut self, _expr: &mut syn::Expr) {
                if let syn::Expr::Macro(i) = _expr {
                    // replace the macro with a block that roughly matches the macro
                    if let Some("rsx" | "render") = i
                        .mac
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
                        let formatted = crate::fmt_block_from_expr(
                            self.writer.raw_src,
                            syn::ExprMacro {
                                attrs: i.attrs.clone(),
                                mac: i.mac.clone(),
                            },
                        )
                        .unwrap();

                        *_expr = syn::parse_quote!(dioxus_autofmt_block__________);

                        // Save this formatted block for later, when we apply it to the original expr
                        self.formatted_stack.push(formatted);
                    }
                }

                syn::visit_mut::visit_expr_mut(self, _expr);
            }
        }

        // Visit the expr and replace the macros with formatted blocks
        let mut replacer = ReplaceMacros {
            writer: self,
            formatted_stack: vec![],
        };

        // builds the expression stack
        let mut modified_expr = expr.clone();
        replacer.visit_expr_mut(&mut modified_expr);

        // now unparsed with the modified expression
        let mut unparsed = unparse_expr(&modified_expr);

        // walk each line looking for the dioxus_autofmt_block__________ token
        // if we find it, replace it with the formatted block
        // if there's indentation we want to presreve it

        // now we can replace the macros with the formatted blocks
        for formatted in replacer.formatted_stack.drain(..) {
            let fmted = if formatted.contains('\n') {
                format!("rsx! {{{formatted}\n}}")
            } else {
                format!("rsx! {{{formatted}}}")
            };
            let mut out_fmt = String::new();
            let mut whitespace = 0;

            for line in unparsed.lines() {
                if line.contains("dioxus_autofmt_block__________") {
                    whitespace = line.chars().take_while(|c| c.is_whitespace()).count();
                    break;
                }
            }

            for (idx, fmt_line) in fmted.lines().enumerate() {
                // Push the indentation
                if idx > 0 {
                    out_fmt.push_str(&" ".repeat(whitespace));
                }

                out_fmt.push_str(fmt_line);

                // Push a newline
                out_fmt.push('\n');
            }

            // Remove the last newline
            out_fmt.pop();

            // Replace the dioxus_autofmt_block__________ token with the formatted block
            unparsed = unparsed.replacen("dioxus_autofmt_block__________", &out_fmt, 1);
            continue;
        }

        unparsed
    }
}

/// Unparse an expression back into a string
///
/// This creates a new temporary file, parses the expression into it, and then formats the file.
/// This is a bit of a hack, but dtonlay doesn't want to support this very simple usecase, forcing us to clone the expr
pub fn unparse_expr(expr: &Expr) -> String {
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
    let expr = syn::parse_str("1 + 1").unwrap();
    let unparsed = unparse(&wrapped(&expr));
    assert_eq!(unparsed, "fn main() {\n    1 + 1;\n}\n");
}

#[test]
fn unparses_completely() {
    let expr = syn::parse_str("1 + 1").unwrap();
    let unparsed = unparse_expr(&expr);
    assert_eq!(unparsed, "1 + 1");
}

#[test]
fn unparses_let_guard() {
    let expr = syn::parse_str("let Some(url) = &link.location").unwrap();
    let unparsed = unparse_expr(&expr);
    assert_eq!(unparsed, "let Some(url) = &link.location");
}

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
