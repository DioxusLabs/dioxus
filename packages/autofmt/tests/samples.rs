macro_rules! twoway {
    (
        $(

            // doc attrs
            $( #[doc = $doc:expr] )*
            $name:ident
        ),*
    ) => {
        $(
            $( #[doc = $doc] )*
            #[test]
            fn $name() {
                let src = include_str!(concat!("./samples/", stringify!($name), ".rsx"));
                let formatted = dioxus_autofmt::fmt_file(src);
                let out = dioxus_autofmt::apply_formats(src, formatted);
                pretty_assertions::assert_eq!(&src, &out);
            }
        )*
    };
}

twoway![
    simple,
    comments,
    attributes,
    manual_props,
    complex,
    tiny,
    tinynoopt,
    long,
    key,
    multirsx,
    commentshard,
    emoji,
    messy_indent,
    long_exprs
];
