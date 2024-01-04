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
    collapse_expr,
    comments,
    commentshard,
    complex,
    emoji,
    ifchain_forloop,
    immediate_expr,
    key,
    long_exprs,
    long,
    manual_props,
    messy_indent,
    multirsx,
    raw_strings,
    reallylong,
    simple,
    t2,
    tiny,
    tinynoopt,
    trailing_expr,
];
