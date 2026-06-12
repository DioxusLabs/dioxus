mod fuzz_support;

use proptest::prelude::*;

fn fuzz_config() -> ProptestConfig {
    ProptestConfig {
        cases: std::env::var("AUTOFMT_FUZZ_ITERS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500),
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(fuzz_config())]

    #[test]
    fn fuzz_random_rsx(data in prop::collection::vec(any::<u8>(), 0..4096)) {
        fuzz_support::run_input(&data);
    }
}

#[test]
fn fuzz_corpus_smoke_test() {
    let corpus = [
        vec![0; 4096],
        vec![u8::MAX; 4096],
        (0..4096).map(|i| i as u8).collect::<Vec<_>>(),
        (0..4096).map(|i| (i * 37 + 11) as u8).collect::<Vec<_>>(),
    ];

    let checked = corpus
        .iter()
        .filter(|input| fuzz_support::run_input(input) == fuzz_support::RunResult::Checked)
        .count();

    assert!(
        checked > 0,
        "smoke corpus did not produce any valid rsx cases"
    );
}
