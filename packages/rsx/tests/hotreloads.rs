use dioxus_rsx::hot_reload::{diff_rsx, DiffResult};
use syn::File;

fn load_files(old: &str, new: &str) -> (File, File) {
    let old = syn::parse_file(old).unwrap();
    let new = syn::parse_file(new).unwrap();
    (old, new)
}

#[test]
fn hotreloads() {
    let (old, new) = load_files(
        include_str!("./valid/expr.old.rsx"),
        include_str!("./valid/expr.new.rsx"),
    );

    assert!(matches!(
        diff_rsx(&new, &old),
        DiffResult::RsxChanged { .. }
    ));

    let (old, new) = load_files(
        include_str!("./valid/let.old.rsx"),
        include_str!("./valid/let.new.rsx"),
    );

    assert!(matches!(
        diff_rsx(&new, &old),
        DiffResult::RsxChanged { .. }
    ));

    let (old, new) = load_files(
        include_str!("./valid/combo.old.rsx"),
        include_str!("./valid/combo.new.rsx"),
    );

    assert!(matches!(
        diff_rsx(&new, &old),
        DiffResult::RsxChanged { .. }
    ));
}

#[test]
fn doesnt_hotreload() {
    let (old, new) = load_files(
        include_str!("./invalid/changedexpr.old.rsx"),
        include_str!("./invalid/changedexpr.new.rsx"),
    );

    let res = diff_rsx(&new, &old);
    dbg!(&res);
    assert!(matches!(res, DiffResult::CodeChanged(_)));
}
