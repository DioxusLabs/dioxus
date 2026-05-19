#![no_main]

use dioxus_vdom_fuzz::{
    FuzzCase, ReductionOptions, decode_case, encode_case, encode_case_vec, format_failure_report,
    print_case_trace, reduce_case, run_case,
};
use libfuzzer_sys::{fuzz_mutator, fuzz_target, fuzzer_mutate};
use mutatis::Session;
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::{Mutex, OnceLock},
};

fuzz_target!(|data: &[u8]| {
    let Some(case) = decode_case(data) else {
        return;
    };

    if let Err(failure) = run_case(&case) {
        print_case_trace(&case, &failure);
        panic!("{}", format_failure_report(&case, &failure));
    }
});

fuzz_mutator!(|data: &mut [u8], size: usize, max_size: usize, seed: u32| {
    let mut case = decode_case(&data[..size]).unwrap_or_else(FuzzCase::seed);
    let minimizing = cargo_fuzz_minimizing();

    if minimizing {
        if let Some(reduced) = cached_semantic_reduction(&case, &data[..size], max_size) {
            data[..reduced.len()].copy_from_slice(&reduced);
            return reduced.len();
        }
    }

    let mut session = Session::new()
        .seed(seed.into())
        .shrink(minimizing || max_size <= size);

    if session.mutate(&mut case).is_err() {
        return fuzzer_mutate(data, size, max_size);
    }

    case.normalize();
    encode_case(&case, data, max_size).unwrap_or_else(|| fuzzer_mutate(data, size, max_size))
});

fn cargo_fuzz_minimizing() -> bool {
    static MINIMIZING: OnceLock<bool> = OnceLock::new();
    *MINIMIZING.get_or_init(|| {
        std::env::args().any(|arg| {
            arg == "-minimize_crash=1"
                || arg == "-minimize_crash"
                || arg == "--minimize_crash=1"
                || arg == "-minimize_crash_internal_step=1"
                || arg == "--minimize_crash_internal_step=1"
        })
    })
}

fn cached_semantic_reduction(
    case: &FuzzCase,
    encoded_case: &[u8],
    max_size: usize,
) -> Option<Vec<u8>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, Option<Vec<u8>>>>> = OnceLock::new();

    let mut hasher = DefaultHasher::new();
    encoded_case.hash(&mut hasher);
    let key = hasher.finish();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(&key).cloned() {
        return cached;
    }

    let reduction = reduce_case(case.clone(), ReductionOptions::default())
        .ok()
        .and_then(|report| {
            let encoded = encode_case_vec(&report.case)?;
            let reduced_ops = report.stats.reduced_ops < report.stats.original_ops;
            let reduced_bytes = encoded.len() < encoded_case.len();
            (encoded.len() <= max_size && (reduced_ops || reduced_bytes)).then_some(encoded)
        });

    cache.lock().unwrap().insert(key, reduction.clone());
    reduction
}
