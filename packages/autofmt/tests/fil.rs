#[test]
fn formats_file_properly() {
    let src = include_str!("./samples/thing.rsxu");

    let formatted = dioxus_autofmt::fmt_file(src);
    let out = dioxus_autofmt::apply_formats(src, formatted);

    println!("{}", out);
}
