const SRC: &str = include_str!("./samples/all.rs");

fn body() -> &'static str {
    &SRC[6..SRC.len() - 3]
}

fn unindented_body() -> String {
    body()
        .lines()
        .map(|line| match line.strip_prefix("    ") {
            Some(line) => line,
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn way_and_back() {
    let blocks = dioxus_autofmt::fmt_file(SRC).into_iter().next().unwrap();

    println!("{}", blocks.formatted);
}
