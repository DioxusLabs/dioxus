//! Dump a fuzz artifact as a structured operation list and run it.
//!
//! Useful for triage: prints the decoded ops, then runs the case with
//! panic-catching so you can see the failure report rather than libfuzzer's
//! bare deadly-signal output.

use dioxus_vdom_fuzz::{decode_case, format_failure_report, print_case_trace, run_case};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: dump_case <artifact-path>");
            return ExitCode::from(2);
        }
    };

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("failed to read {path}: {err}");
            return ExitCode::from(2);
        }
    };

    let Some(case) = decode_case(&bytes) else {
        eprintln!("failed to decode case from {path}");
        return ExitCode::from(2);
    };

    match run_case(&case) {
        Ok(()) => {
            println!("case ran clean ({} ops)", bytes.len());
            ExitCode::SUCCESS
        }
        Err(failure) => {
            print_case_trace(&case, &failure);
            println!("{}", format_failure_report(&case, &failure));
            ExitCode::FAILURE
        }
    }
}
