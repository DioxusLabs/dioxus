dioxus_test_harness::main!();

#[dioxus_test_harness::test]
fn adds() {
    assert_eq!(std::hint::black_box(1) + 1, 2);
}

#[dioxus_test_harness::test]
async fn async_ok() {}

#[dioxus_test_harness::test(ignore)]
fn skipped() {}

#[dioxus_test_harness::test(should_panic)]
fn panics() {
    panic!("expected panic");
}

#[dioxus_test_harness::test(tags = ["ui"])]
fn renders() {
    let mut dom = dioxus::prelude::VirtualDom::new(|| dioxus::prelude::rsx! { "hello" });
    dom.rebuild_in_place();
}

#[dioxus_test_harness::test]
fn fails() {
    assert_eq!(std::hint::black_box(1) + 1, 3);
}
