//! Minimize a libFuzzer artifact via simple greedy bisection: progressively halve
//! the case and try to remove each chunk, then converge by single-op deletion.
//!
//! Usage:
//!   RUSTFLAGS="--cfg fuzzing" \
//!     cargo run --release --example reduce_artifact -p dioxus-vdom-fuzz -- <artifact>

use std::{
    env, fs,
    process::ExitCode,
    time::{Duration, Instant},
};

use dioxus_vdom_fuzz::{
    FuzzFailure, decode_case, encode_case_vec, format_failure_report, print_case_trace, run_case,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: reduce_artifact <artifact-path>");
        return ExitCode::from(2);
    };
    let time_budget =
        Duration::from_secs(args.get(2).and_then(|s| s.parse().ok()).unwrap_or(120u64));

    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("failed to read {path}: {err}");
            return ExitCode::from(2);
        }
    };
    let Some(case) = decode_case(&bytes) else {
        eprintln!("could not decode case from {path}");
        return ExitCode::from(2);
    };

    let Err(original_failure) = run_case(&case) else {
        eprintln!("input does not reproduce a fuzz failure under cfg=fuzzing");
        return ExitCode::from(2);
    };
    let target = signature(&original_failure);
    eprintln!(
        "original: {} ops, fails at step {}: {}",
        case.len(),
        original_failure.step(),
        target
    );

    let mut case = case;
    let started = Instant::now();
    let mut attempts = 0u32;

    // 1) Truncate beyond the failing step.
    let cutoff = original_failure.step() + 1;
    if cutoff < case.len() {
        let candidate = case.truncated(cutoff);
        attempts += 1;
        if let Err(f) = run_case(&candidate) {
            if signature(&f) == target {
                eprintln!("truncate: {} -> {} ops", case.len(), candidate.len());
                case = candidate;
            }
        }
    }

    // 2) Chunk deletion at decreasing granularity.
    let mut chunk = case.len();
    while chunk > 1 && started.elapsed() < time_budget {
        chunk = (chunk / 2).max(1);
        let mut start = 0;
        while start < case.len() && started.elapsed() < time_budget {
            let end = (start + chunk).min(case.len());
            if end - start == case.len() {
                break;
            }
            let candidate = case.without_range(start, end);
            attempts += 1;
            match run_case(&candidate) {
                Err(f) if signature(&f) == target => {
                    eprintln!(
                        "chunk -{} at {}: {} -> {} ops",
                        end - start,
                        start,
                        case.len(),
                        candidate.len()
                    );
                    case = candidate;
                    // don't advance — chunk shrunk the suffix
                }
                _ => start += chunk,
            }
        }
    }

    // 3) Single-op deletion to convergence.
    let mut progress = true;
    while progress && started.elapsed() < time_budget {
        progress = false;
        let mut i = 0;
        while i < case.len() && started.elapsed() < time_budget {
            let candidate = case.without_op(i);
            attempts += 1;
            match run_case(&candidate) {
                Err(f) if signature(&f) == target => {
                    eprintln!("remove [{}]: {} -> {} ops", i, case.len(), candidate.len());
                    case = candidate;
                    progress = true;
                }
                _ => i += 1,
            }
        }
    }

    let final_failure = run_case(&case).unwrap_err();
    let reduced_bytes = encode_case_vec(&case).expect("encode reduced case");
    let out_path = format!("{path}.reduced");
    fs::write(&out_path, &reduced_bytes).expect("write reduced");

    println!();
    println!(
        "reduced to {} ops in {:.1}s after {} attempts",
        case.len(),
        started.elapsed().as_secs_f32(),
        attempts
    );
    println!("written: {out_path}");
    println!();
    print_case_trace(&case, &final_failure);
    println!();
    println!("{}", format_failure_report(&case, &final_failure));

    ExitCode::SUCCESS
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

fn signature(failure: &FuzzFailure) -> String {
    first_line(failure.message()).to_string()
}
