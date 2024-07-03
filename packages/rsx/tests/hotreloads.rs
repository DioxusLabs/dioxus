use dioxus_rsx::hot_reload::{diff_rsx, DiffResult};
use syn::File;

macro_rules! assert_rsx_changed {
    (
        $( #[doc = $doc:expr] )*
        $name:ident
    ) => {
        $( #[doc = $doc] )*
        #[test]
        fn $name() {
            let old = include_str!(concat!("./valid/", stringify!($name), ".old.rsx"));
            let new = include_str!(concat!("./valid/", stringify!($name), ".new.rsx"));
            let (old, new) = load_files(old, new);
            assert!(matches!( diff_rsx(&new, &old), DiffResult::RsxChanged { .. }));
        }
    };
}

macro_rules! assert_code_changed {
    (
        $( #[doc = $doc:expr] )*
        $name:ident
    ) => {
        $( #[doc = $doc] )*
        #[test]
        fn $name() {
            let old = include_str!(concat!("./invalid/", stringify!($name), ".old.rsx"));
            let new = include_str!(concat!("./invalid/", stringify!($name), ".new.rsx"));
            let (old, new) = load_files(old, new);
            assert!(matches!(diff_rsx(&new, &old), DiffResult::CodeChanged(_) ));
        }
    };
}

fn load_files(old: &str, new: &str) -> (File, File) {
    let old = syn::parse_file(old).unwrap();
    let new = syn::parse_file(new).unwrap();
    (old, new)
}

assert_rsx_changed![combo];
assert_rsx_changed![expr];
assert_rsx_changed![for_];
assert_rsx_changed![if_];
assert_rsx_changed![let_];
assert_rsx_changed![nested];

assert_code_changed![changedexpr];
