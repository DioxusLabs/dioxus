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
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

const INTERNAL_MINIMIZE_RANDOM_ATTEMPTS: usize = 64;
const INTERNAL_MINIMIZE_ATTEMPT_LIMIT: usize = 64;

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

    if let Some(options) = cargo_fuzz_semantic_reduction_options() {
        if claim_semantic_reduction_attempt() {
            if let Some(reduced) =
                cached_semantic_reduction(&case, &data[..size], max_size, options)
            {
                data[..reduced.len()].copy_from_slice(&reduced);
                return reduced.len();
            }
        }
    }

    let mut session = Session::new()
        .seed(seed.into())
        .shrink(minimizing || max_size <= size);

    if session.mutate(&mut case).is_err() {
        return fuzzer_mutate(data, size, max_size);
    }

    if minimizing {
        for _ in 0..extra_minimization_mutations(seed) {
            if session.mutate(&mut case).is_err() {
                break;
            }
        }
    }

    case.normalize();
    encode_case(&case, data, max_size).unwrap_or_else(|| fuzzer_mutate(data, size, max_size))
});

fn extra_minimization_mutations(seed: u32) -> usize {
    let mut state = seed as u64 ^ 0x9E37_79B9_7F4A_7C15;
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);

    if state & 0b11 == 0 {
        1 + ((state >> 8) as usize % 7)
    } else {
        0
    }
}

fn cargo_fuzz_minimizing() -> bool {
    static MINIMIZING: OnceLock<bool> = OnceLock::new();
    *MINIMIZING.get_or_init(|| std::env::args().any(|arg| is_minimize_crash_arg(&arg)))
}

fn claim_semantic_reduction_attempt() -> bool {
    static ATTEMPTED: AtomicBool = AtomicBool::new(false);
    !ATTEMPTED.swap(true, Ordering::Relaxed)
}

fn cargo_fuzz_semantic_reduction_options() -> Option<ReductionOptions> {
    static OPTIONS: OnceLock<Option<ReductionOptions>> = OnceLock::new();
    OPTIONS
        .get_or_init(|| {
            let mut minimizing = false;
            let mut internal_step = false;
            for arg in std::env::args() {
                if is_minimize_crash_internal_step_arg(&arg) {
                    internal_step = true;
                }
                minimizing |= is_minimize_crash_arg(&arg);
            }

            if !minimizing {
                return None;
            }

            let options = if internal_step {
                ReductionOptions::default()
                    .random_multi_attempts(INTERNAL_MINIMIZE_RANDOM_ATTEMPTS)
                    .max_attempts(INTERNAL_MINIMIZE_ATTEMPT_LIMIT)
            } else {
                ReductionOptions::default()
            };

            Some(options)
        })
        .clone()
}

fn is_minimize_crash_arg(arg: &str) -> bool {
    matches!(
        arg,
        "-minimize_crash=1"
            | "-minimize_crash"
            | "--minimize_crash=1"
            | "-minimize_crash_internal_step=1"
            | "--minimize_crash_internal_step=1"
    )
}

fn is_minimize_crash_internal_step_arg(arg: &str) -> bool {
    matches!(
        arg,
        "-minimize_crash_internal_step=1" | "--minimize_crash_internal_step=1"
    )
}

fn cached_semantic_reduction(
    case: &FuzzCase,
    encoded_case: &[u8],
    max_size: usize,
    options: ReductionOptions,
) -> Option<Vec<u8>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, Option<Vec<u8>>>>> = OnceLock::new();

    let mut hasher = DefaultHasher::new();
    encoded_case.hash(&mut hasher);
    let key = hasher.finish();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(&key).cloned() {
        return cached;
    }

    let reduction = reduce_case(case.clone(), options).ok().and_then(|report| {
        let encoded = encode_case_vec(&report.case)?;
        let reduced_ops = report.stats.reduced_ops < report.stats.original_ops;
        let reduced_bytes = encoded.len() < encoded_case.len();
        (encoded.len() <= max_size && (reduced_ops || reduced_bytes)).then_some(encoded)
    });

    cache.lock().unwrap().insert(key, reduction.clone());
    reduction
}
