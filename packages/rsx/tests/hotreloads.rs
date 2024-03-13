use dioxus_rsx::hot_reload::diff_rsx;
use syn::File;

fn load_files(old: &str, new: &str) -> (File, File) {
    let old = syn::parse_file(old).unwrap();
    let new = syn::parse_file(new).unwrap();
    (old, new)
}

#[test]
fn hotreloads() {
    let (old, new) = load_files(
        include_str!("./valid_samples/old.expr.rsx"),
        include_str!("./valid_samples/new.expr.rsx"),
    );

    let res = diff_rsx(&new, &old);
    dbg!(res);
}
