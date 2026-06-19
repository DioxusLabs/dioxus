//! Reusable Dioxus VirtualDom fuzzing harness.
//!
//! The `cargo-fuzz` target feeds encoded [`FuzzCase`] values into this crate.
//! LibFuzzer owns coverage guidance and corpus management; this crate owns the
//! structured operation stream and renderer oracle.
//!
//! Module map:
//! - [`case`]: the encoded op stream, replay, and failure reporting
//! - [`ops`]: the operation grammar and how ops apply to the model
//! - [`model`]: the spec tree the generated app renders from
//! - [`mutator`]: structure-aware mutation (the op strategy table)
//! - [`reducer`]: structured shrinking of failing cases
//! - [`harness`]: incremental-vs-fresh renderer oracle and lifecycle checks
//! - [`vdom`]: compiles model specs into real `VNode`s/`Template`s
//! - [`warmup`]: one-shot scenarios for paths replay cannot reach
//!
//! The crate compiles in ordinary builds so CI type-checks it and runs the
//! [`targeted`] regression recipes under `cargo test`. The libFuzzer binary
//! (`packages/fuzz/fuzz`) sets `--cfg fuzzing`, which only flips runtime
//! behavior via `cfg!(fuzzing)` (e.g. strict-by-default oracle options).
#![deny(unsafe_code)]

mod cache;
mod case;
mod context;
mod diagnostics;
mod event;
mod harness;
mod lifecycle;
mod model;
mod mutator;
mod ops;
mod reducer;
#[cfg(test)]
mod targeted;
mod vdom;
mod warmup;

pub use case::{
    FuzzCase, FuzzFailure, decode_case, encode_case, format_failure_report, print_case_trace,
    run_case,
};
pub use mutator::mutate_case;
pub use reducer::{ReductionOptions, reduce_case_to_encoded_vec};
pub use warmup::warmup_deferred_priority_paths;
