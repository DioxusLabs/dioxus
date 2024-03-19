use prettyplease::unparse;
use syn::{Expr, File, Item};

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
    raw.strip_prefix("fn main() {\n")
        .unwrap()
        .strip_suffix("}\n")
        .unwrap()
        .lines()
        .map(|line| line.strip_prefix("    ").unwrap()) // todo: set this to tab level
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrapped(expr: &Expr) -> File {
    File {
        shebang: None,
        attrs: vec![],
        items: vec![
            //
            Item::Verbatim(quote::quote! {
                fn main() {
                    #expr
                }
            }),
        ],
    }
}

#[test]
fn unparses_raw() {
    let expr = syn::parse_str("1 + 1").unwrap();
    let unparsed = unparse(&wrapped(&expr));
    assert_eq!(unparsed, "fn main() {\n    1 + 1\n}\n");
}

#[test]
fn unparses_completely() {
    let expr = syn::parse_str("1 + 1").unwrap();
    let unparsed = unparse_expr(&expr);
    assert_eq!(unparsed, "1 + 1");
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
