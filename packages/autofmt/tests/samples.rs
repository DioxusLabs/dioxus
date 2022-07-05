macro_rules! twoway {
    ($val:literal => $name:ident) => {
        #[test]
        fn $name() {
            let src = include_str!(concat!("./samples/", $val, ".rsx"));
            let formatted = dioxus_autofmt::fmt_file(src);
            let out = dioxus_autofmt::apply_formats(src, formatted);
            pretty_assertions::assert_eq!(&src, &out);
        }
    };
}

twoway! ("simple" => simple);

twoway! ("comments" => comments);

twoway! ("attributes" => attributes);

twoway! ("manual_props" => manual_props);

twoway! ("complex" => complex);

twoway! ("tiny" => tiny);

twoway! ("tinynoopt" => tinynoopt);
