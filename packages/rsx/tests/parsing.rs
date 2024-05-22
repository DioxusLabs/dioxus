use dioxus_rsx::{hot_reload::Empty, CallBody};
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::Item;

use dioxus_rsx::PrettyUnparse;

#[test]
fn rsx_writeout_snapshot() {
    let body = parse_from_str(include_str!("./parsing/multiexpr.rsx"));

    assert_eq!(body.body.roots.len(), 1);

    let root = &body.body.roots[0];

    let el = match root {
        dioxus_rsx::BodyNode::Element(el) => el,
        _ => panic!("Expected an element"),
    };

    assert_eq!(el.name, "circle");

    assert_eq!(el.raw_attributes.len(), 5);

    // let mut context = DynamicContext::default();
    // let o = context.render_static_node(&body.roots[0]);

    // hi!!!!!
    // you're probably here because you changed something in how rsx! generates templates and need to update the snapshot
    // This is a snapshot test. Make sure the contents are checked before committing a new snapshot.
    // let stability_tested = o.to_string();
    // assert_eq!(
    //     stability_tested.trim(),
    //     include_str!("./parsing/multiexpr.expanded.rsx").trim()
    // );
}

fn parse_from_str(contents: &str) -> CallBody {
    // Parse the file
    let file = syn::parse_file(contents).unwrap();

    // The first token should be the macro call
    let Item::Macro(call) = file.items.first().unwrap() else {
        panic!("Expected a macro call");
    };

    call.mac.parse_body().unwrap()
}

/// are spans just byte offsets? can't we just use the byte offset relative to the root?
#[test]
fn how_do_spans_work_again() {
    fn print_spans(item: TokenStream) {
        let new_invalid: CallBody = syn::parse2(item).unwrap();
        let root = &new_invalid.body.roots[0];
        let hi = &new_invalid.body.roots[0].children()[0];
        let goodbye = &new_invalid.body.roots[0].children()[1];

        dbg!(root.span(), hi.span(), goodbye.span());
        dbg!(
            root.span().start(),
            hi.span().start(),
            goodbye.span().start()
        );
        dbg!(root, hi, goodbye);

        // dbg!(second.span());
        // dbg!(first);
        // let third = new_invalid.roots[0].children().first().unwrap();
        // dbg!(third.span());
        // let last = new_invalid.roots.last().unwrap().children().last().unwrap();
        // dbg!(last.span());
        println!();
    }

    for _ in 0..5 {
        print_spans(quote::quote! {
            div {
                h1 {}
                for item in items {}
                // something-cool {}
                // if true {
                //     div {}
                // }
                "hi!"
                "goodbye!"
            }
        });
    }

    let span = Span::call_site();
    dbg!(span.start(), span.end());
}

#[test]
fn spans_from_files() {
    let contents = include_str!("./parsing/multiexpr.rsx");
    let file = syn::parse_file(contents).unwrap();
}

#[test]
fn callbody_ctx() {
    let item = quote::quote! {
        div {
            h1 {
                id: "Some cool attribute {cool}"
            }
            for item in items {
                "Something {cool}"
            }
            Component {
                "Something {elseish}"
            }
            Component2 {
                "Something {Body}"
                Component3 {
                    prop: "Something {Body3}",
                    "Something {Body4}"
                }
            }
            something-cool {
                "Something {cool}ish"
            }
            if true {
                div {
                    "hi! {cool}"
                }
            }
            "hi!"
            "goodbye!"
        }
        {some_expr}
    };

    let cb: CallBody = syn::parse2(item).unwrap();

    dbg!(cb.tempalte_idx.get());
    dbg!(cb.ifmt_idx.get());

    let template = cb.body.to_template::<Empty>();

    // for attr in body.attr_paths.iter() {
    //     dbg!(body.get_dyn_attr(&attr));
    // }

    // for root in body.node_paths.iter() {
    //     dbg!(body.get_dyn_node(&root));
    // }
}

#[test]
fn simple_case() {
    let item = quote::quote! {
        div {
            something: "cool",
            id: "Some cool attribute {cool}",
            class: "Some cool attribute {cool2}",
            "hi!"
            {some_expr}
            Component {
                boolish: true,
                otherish: 123,
                otherish2: 123.0,
                otherish3: "dang!",
                otherish3: "dang! {cool}",
            }
        }
    };

    let cb: CallBody = syn::parse2(item).unwrap();
    println!("{}", cb.to_token_stream().pretty_unparse());
}
