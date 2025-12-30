#![allow(deprecated)]

use dioxus_autofmt::{IndentOptions, IndentType};

macro_rules! twoway {
    ($val:literal => $name:ident ($indent:expr)) => {
        #[test]
        fn $name() {
            let src_right = include_str!(concat!("./wrong/", $val, ".rsx"));
            let src_wrong = include_str!(concat!("./wrong/", $val, ".wrong.rsx"));

            let parsed = syn::parse_file(src_wrong)
                .expect("fmt_file should only be called on valid syn::File files");

            let formatted =
                dioxus_autofmt::try_fmt_file(src_wrong, &parsed, $indent).unwrap_or_default();
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
twoway!("multiexpr-many" => multiexpr_many (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("simple-combo-expr" => simple_combo_expr (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("oneline-expand" => online_expand (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("shortened" => shortened (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("syntax_error" => syntax_error (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("skipfail" => skipfail (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("comments-inline-4sp" => comments_inline_4sp (IndentOptions::new(IndentType::Spaces, 4, false)));
twoway!("comments-attributes-4sp" => comments_attributes_4sp (IndentOptions::new(IndentType::Spaces, 4, false)));
