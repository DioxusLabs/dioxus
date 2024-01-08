#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/01-simple.rs");
}
