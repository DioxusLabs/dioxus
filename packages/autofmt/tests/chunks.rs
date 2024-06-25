use dioxus_autofmt::IndentOptions;

#[test]
fn what_chunks_does_autofmt_produce() {
    let file = include_str!("../tests/samples/misplaced.rsx");

    let chunks = dioxus_autofmt::fmt_file(file, IndentOptions::default());
    dbg!(chunks);

    // let file = syn::parse_file(file).unwrap();

    // let mut macros = vec![];
    // dioxus_autofmt::collect_macros::collect_from_file(&file, &mut macros);

    // // for m in macros {
    // //     dbg!(m);
    // // }
}
