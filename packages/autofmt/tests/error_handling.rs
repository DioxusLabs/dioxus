#[test]
fn no_parse() {
    let src = include_str!("./partials/no_parse.rsx");
    assert!(syn::parse_file(src).is_err());
}

#[test]
fn parses_but_fmt_fails() {
    let src = include_str!("./partials/wrong.rsx");
    let file = syn::parse_file(src).unwrap();
    let formatted = dioxus_autofmt::try_fmt_file(src, &file, Default::default());
    assert!(&formatted.is_err());
}

#[test]
fn parses_and_is_okay() {
    let src = include_str!("./partials/okay.rsx");
    let file = syn::parse_file(src).unwrap();
    let formatted = dioxus_autofmt::try_fmt_file(src, &file, Default::default()).unwrap();
    assert_ne!(formatted.len(), 0);
}
