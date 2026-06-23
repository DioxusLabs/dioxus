#![allow(deprecated)]

macro_rules! twoway {
    (
        $(

            // doc attrs
            $( #[doc = $doc:expr] )*
            $name:ident,
        )*
    ) => {
        $(
            $( #[doc = $doc] )*
            #[test]
            fn $name() {
                let src = include_str!(concat!("./samples/", stringify!($name), ".rsx"));
                let formatted = dioxus_autofmt::fmt_file(src, Default::default());
                let out = dioxus_autofmt::apply_formats(src, formatted);
                // normalize line endings
                let out = out.replace("\r", "");
                let src = src.replace("\r", "");
                pretty_assertions::assert_eq!(&src, &out);
            }
        )*
    };
}
twoway![
    attributes,
    basic_expr,
    collapse_expr,
    comments_attr_expr_blocks,
    comments,
    comments_async_closure,
    comments_expr_with_strings,
    comments_nested_closures,
    commentshard,
    complex,
    docsite,
    emoji,
    fat_exprs,
    ifchain_forloop,
    immediate_expr,
    key,
    letsome,
    long_exprs,
    long,
    manual_props,
    many_exprs,
    messy_indent,
    misplaced,
    multirsx,
    nested,
    raw_strings,
    reallylong,
    shorthand,
    simple,
    skip,
    spaces,
    staged,
    t2,
    tiny,
    tinynoopt,
    trailing_expr,
    oneline,
    prop_rsx,
    asset,
    collapse,
    expr_on_conditional,
    blank_lines,
    blank_lines_preserved,
    forloop_tuple,
    commented_rsx_block,
    commented_rsx_block_nested,
    commented_rsx_block_only,
    commented_rsx_block_between,
    commented_rsx_block_deep,
    long_if_else_attr,
    empty_component_body,
    empty_braces_oneliner,
];

fn assert_idempotent(src: &str) {
    let src = src.replace("\r", "");

    let once =
        dioxus_autofmt::apply_formats(&src, dioxus_autofmt::fmt_file(&src, Default::default()))
            .replace("\r", "");
    let twice =
        dioxus_autofmt::apply_formats(&once, dioxus_autofmt::fmt_file(&once, Default::default()))
            .replace("\r", "");

    pretty_assertions::assert_eq!(src, once);
    pretty_assertions::assert_eq!(once, twice);
}

#[test]
fn comments_async_closure_is_idempotent() {
    assert_idempotent(include_str!("./samples/comments_async_closure.rsx"));
}

#[test]
fn comments_nested_closures_is_idempotent() {
    assert_idempotent(include_str!("./samples/comments_nested_closures.rsx"));
}

#[test]
fn comments_attr_expr_blocks_is_idempotent() {
    assert_idempotent(include_str!("./samples/comments_attr_expr_blocks.rsx"));
}

#[test]
fn comments_expr_with_strings_is_idempotent() {
    assert_idempotent(include_str!("./samples/comments_expr_with_strings.rsx"));
}

#[test]
fn long_if_else_attr_is_idempotent() {
    assert_idempotent(include_str!("./samples/long_if_else_attr.rsx"));
}

#[test]
fn empty_component_body_is_idempotent() {
    assert_idempotent(include_str!("./samples/empty_component_body.rsx"));
}

#[test]
fn empty_braces_match_arm_is_idempotent() {
    let src = include_str!("./samples/empty_braces_match_arm.rsx");
    let once =
        dioxus_autofmt::apply_formats(src, dioxus_autofmt::fmt_file(src, Default::default()));
    let twice =
        dioxus_autofmt::apply_formats(&once, dioxus_autofmt::fmt_file(&once, Default::default()));
    let thrice =
        dioxus_autofmt::apply_formats(&twice, dioxus_autofmt::fmt_file(&twice, Default::default()));
    pretty_assertions::assert_eq!(&once, &twice, "pass 1 vs pass 2");
    pretty_assertions::assert_eq!(&twice, &thrice, "pass 2 vs pass 3");
}

#[test]
fn empty_braces_no_space_is_idempotent() {
    let src = "rsx! { Router::<Route>{}}";
    let once =
        dioxus_autofmt::apply_formats(src, dioxus_autofmt::fmt_file(src, Default::default()));
    let twice =
        dioxus_autofmt::apply_formats(&once, dioxus_autofmt::fmt_file(&once, Default::default()));
    let thrice =
        dioxus_autofmt::apply_formats(&twice, dioxus_autofmt::fmt_file(&twice, Default::default()));
    eprintln!("=== ONCE ===\n{once}");
    eprintln!("=== TWICE ===\n{twice}");
    eprintln!("=== THRICE ===\n{thrice}");
    pretty_assertions::assert_eq!(&once, &twice, "pass 1 vs pass 2");
    pretty_assertions::assert_eq!(&twice, &thrice, "pass 2 vs pass 3");
}

#[test]
fn empty_braces_oneliner_is_idempotent() {
    let src = r#"rsx! { Router::<Route>{}}"#;
    let once =
        dioxus_autofmt::apply_formats(src, dioxus_autofmt::fmt_file(src, Default::default()));
    let twice =
        dioxus_autofmt::apply_formats(&once, dioxus_autofmt::fmt_file(&once, Default::default()));
    let thrice =
        dioxus_autofmt::apply_formats(&twice, dioxus_autofmt::fmt_file(&twice, Default::default()));
    pretty_assertions::assert_eq!(&once, &twice, "pass 1 vs pass 2");
    pretty_assertions::assert_eq!(&twice, &thrice, "pass 2 vs pass 3");
}
