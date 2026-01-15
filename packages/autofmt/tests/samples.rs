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
    comments,
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
];
