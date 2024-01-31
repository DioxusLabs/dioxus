use dioxus_autofmt::{IndentOptions, IndentType};

macro_rules! twoway {
    ($val:literal => $name:ident ($indent:expr)) => {
        #[test]
        fn $name() {
            let src_right = include_str!(concat!("./wrong/", $val, ".rsx"));
            let src_wrong = include_str!(concat!("./wrong/", $val, ".wrong.rsx"));
            let formatted = dioxus_autofmt::fmt_file(src_wrong, $indent);
            let out = dioxus_autofmt::apply_formats(src_wrong, formatted);

            // normalize line endings
            let out = out.replace("\r", "");
            let src_right = src_right.replace("\r", "");

            pretty_assertions::assert_eq!(&src_right, &out);
        }
    };
}

twoway!("comments-4sp" => comments_4sp (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("comments-tab" => comments_tab (IndentOptions::new(IndentType::Tabs, 4, false)));

twoway!("multi-4sp" => multi_4sp (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("multi-tab" => multi_tab (IndentOptions::new(IndentType::Tabs, 4, false)));

twoway!("multiexpr-4sp" => multiexpr_4sp (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("multiexpr-tab" => multiexpr_tab (IndentOptions::new(IndentType::Tabs, 4, false)));
