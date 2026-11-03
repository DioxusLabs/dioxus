//! The encoded fuzz case: a bounded stream of [`Op`]s, plus replay and
//! failure reporting.

use crate::diagnostics::panic_message;
use crate::harness::{Harness, apply_step, print_ssr_diff_trace};
use crate::ops::Op;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    panic::{self, AssertUnwindSafe},
};

/// Upper bound on the number of operations a single case may replay, so
/// mutated corpus inputs cannot produce unbounded work.
pub(crate) const MAX_STEPS: usize = 512;

pub struct FuzzCase {
    pub(crate) ops: Vec<Op>,
}

impl FuzzCase {
    pub(crate) fn new(mut ops: Vec<Op>) -> Self {
        ops.truncate(MAX_STEPS);
        Self { ops }
    }

    pub(crate) fn normalize(&mut self) {
        self.ops.truncate(MAX_STEPS);
    }

    pub(crate) fn clone_case(&self) -> Self {
        Self {
            ops: self.ops.clone(),
        }
    }
}

impl Default for FuzzCase {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[derive(Serialize)]
struct EncodedFuzzCase<'a> {
    ops: &'a [Op],
}

#[derive(Deserialize)]
struct DecodedFuzzCase {
    ops: Vec<Op>,
}

pub fn decode_case(data: &[u8]) -> Option<FuzzCase> {
    let decoded = postcard::from_bytes::<DecodedFuzzCase>(data).ok()?;
    Some(FuzzCase::new(decoded.ops))
}

pub fn encode_case(case: &FuzzCase, data: &mut [u8], max_size: usize) -> Option<usize> {
    let size = max_size.min(data.len());
    let encoded =
        postcard::to_slice(&EncodedFuzzCase { ops: &case.ops }, &mut data[..size]).ok()?;
    Some(encoded.len())
}

pub(crate) fn encode_case_vec(case: &FuzzCase) -> Option<Vec<u8>> {
    postcard::to_allocvec(&EncodedFuzzCase { ops: &case.ops }).ok()
}

#[derive(Debug)]
pub struct FuzzFailure {
    pub(crate) step: usize,
    pub(crate) op: String,
    pub(crate) message: String,
}

impl fmt::Display for FuzzFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let summary = self.message.lines().next().unwrap_or(&self.message);
        write!(
            f,
            "fuzz case failed at step {} while applying {}: {}",
            self.step, self.op, summary
        )
    }
}

pub fn run_case(case: &FuzzCase) -> Result<(), FuzzFailure> {
    let mut state =
        panic::catch_unwind(AssertUnwindSafe(Harness::fresh)).map_err(|payload| FuzzFailure {
            step: 0,
            op: "<initial rebuild>".to_string(),
            message: format!(
                "panic before applying operation: {}",
                panic_message(&payload)
            ),
        })?;

    for (step, op) in case.ops.iter().enumerate() {
        let applied = panic::catch_unwind(AssertUnwindSafe(|| apply_step(&mut state, op)))
            .map_err(|payload| FuzzFailure {
                step,
                op: format!("{op:?}"),
                message: format!(
                    "panic while applying operation: {}",
                    panic_message(&payload)
                ),
            })?;

        applied.map_err(|message| FuzzFailure {
            step,
            op: format!("{op:?}"),
            message,
        })?;
    }
    Ok(())
}

pub fn format_failure_report(case: &FuzzCase, failure: &FuzzFailure) -> String {
    let mut report = String::new();
    let summary = failure.message.lines().next().unwrap_or(&failure.message);

    use fmt::Write;
    writeln!(&mut report, "fuzz failure").unwrap();
    writeln!(&mut report, "decoded operations: {}", case.ops.len()).unwrap();
    writeln!(&mut report, "failed at step: {}", failure.step).unwrap();
    writeln!(&mut report, "failing op: {}", failure.op).unwrap();
    writeln!(&mut report, "summary: {summary}").unwrap();
    writeln!(&mut report).unwrap();
    writeln!(&mut report, "operations:").unwrap();
    for (index, op) in case.ops.iter().enumerate() {
        let marker = if index == failure.step { ">>" } else { "  " };
        writeln!(&mut report, "{marker} {index:03}: {op:?}").unwrap();
    }
    writeln!(&mut report).unwrap();
    writeln!(&mut report, "full error:").unwrap();
    for line in failure.message.lines() {
        writeln!(&mut report, "  {line}").unwrap();
    }

    report
}

pub fn print_case_trace(case: &FuzzCase, failure: &FuzzFailure) {
    print_ssr_diff_trace(&case.ops, failure.step, &failure.message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_case_roundtrips_and_replays() {
        let case = FuzzCase::default();
        let mut bytes = [0; 4096];
        let size = encode_case(&case, &mut bytes, 4096).unwrap();
        let decoded = decode_case(&bytes[..size]).unwrap();
        assert_eq!(encode_case_vec(&case), encode_case_vec(&decoded));
        run_case(&decoded).unwrap();
    }
}
