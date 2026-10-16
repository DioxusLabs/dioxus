dioxus_test_harness::main!();

#[dioxus_test_harness::test]
fn pass() {
    assert_eq!(std::hint::black_box(2) + 2, 4);
}

#[dioxus_test_harness::test(ignore)]
fn ignored() {
    panic!("ignored");
}

#[dioxus_test_harness::test(should_panic)]
fn expected_panic() {
    panic!("expected");
}

#[dioxus_test_harness::test]
async fn async_pass() {}
