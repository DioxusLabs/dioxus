macro_rules! twoway {
    ($val:literal => $name:ident) => {
        #[test]
        fn $name() {
            let src_right = include_str!(concat!("./wrong/", $val, ".rsx"));
            let src_wrong = include_str!(concat!("./wrong/", $val, ".wrong.rsx"));
            let formatted = dioxus_autofmt::fmt_file(src_wrong);
            let out = dioxus_autofmt::apply_formats(src_wrong, formatted);

            // normalize line endings
            let out = out.replace("\r", "");
            let src_right = src_right.replace("\r", "");

            pretty_assertions::assert_eq!(&src_right, &out);
        }
    };
}

twoway!("comments" => comments);

twoway!("multi" => multi);

twoway!("multiexpr" => multiexpr);
