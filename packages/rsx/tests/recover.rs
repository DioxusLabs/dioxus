use dioxus_rsx::rsx_parser::ParsedRsx;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::ParseStream;

fn parsed(item: TokenStream) -> ParsedRsx {
    let new_invalid: ParsedRsx = syn::parse2(item).unwrap();
    new_invalid
}

fn dbged(item: TokenStream) {
    dbg!(parsed(item));
}

#[test]
fn simple_cases_pass() {
    let out = parsed(quote! {
        div {
            div {
                "hi!"
                div {}
                "hi "
            }
            Component {}
        }
        "hi"
        div {}
    });

    dbg!(out.roots);
}

#[test]
fn basic_expansion() {
    dbged(quote! {
        div {}
        di
    });

    dbged(quote! {
        div {}
        Comp
    });
}

// partial expand to
// div { crate::dioxus:: }
// VComponent::new(crate::dioxus::|,)
#[test]
fn partial_parse_components() {
    // The last node in a block
    dbged(quote! { some::cool:: });
    dbged(quote! {
        div { some::cool:: }
    });

    // Missing curly braces
    dbged(quote! { some::cool::Something });

    // Completely valid
    dbged(quote! { some::cool::Something {} });

    // Complex 1
    dbged(quote! {
        div {}
        some::cool::I
    });

    // Complex 2
    dbged(quote! {
        div {
            some::cool::Thing
            div
        }
    });

    // Incomlete exprs
    dbged(quote! {
        div {
            {some.}
        }
    });

    // // Complete failure
    // dbged(quote! {
    //     some::cool::$
    // });
}

