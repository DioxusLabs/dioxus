//! Reusable Dioxus VirtualDom fuzzing harness.
//!
//! The `cargo-fuzz` target feeds encoded [`FuzzCase`] values into this crate.
//! LibFuzzer owns coverage guidance and corpus management; this crate owns the
//! structured operation stream and renderer oracle.

mod harness;
mod model;
mod ops;
mod reducer;
mod vdom;

use harness::{Harness, apply_step, print_ssr_diff_trace};
use mutatis::{Candidates, DefaultMutate, Generate, Mutate, Result as MutatisResult};
use ops::{IteratorScenario, Op};
pub use reducer::{ReduceError, ReductionOptions, ReductionReport, ReductionStats, reduce_case};
use reducer::{random_multistep_shrink_case, simplified_ops};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_STEPS: usize = 256;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FuzzCase {
    ops: Vec<Op>,
}

impl FuzzCase {
    pub(crate) fn new(mut ops: Vec<Op>) -> Self {
        ops.truncate(MAX_STEPS);
        Self { ops }
    }

    pub fn seed() -> Self {
        let ops = IteratorScenario::ALL
            .into_iter()
            .enumerate()
            .flat_map(|(index, scenario)| {
                ops::iterator_scenario_ops(scenario, (index as u8).wrapping_mul(16))
            })
            .collect();
        Self::new(ops)
    }

    pub fn normalize(&mut self) {
        self.ops.truncate(MAX_STEPS);
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

impl Default for FuzzCase {
    fn default() -> Self {
        Self::seed()
    }
}

#[derive(Clone, Debug, Default)]
pub struct FuzzCaseMutator;

impl DefaultMutate for FuzzCase {
    type DefaultMutate = FuzzCaseMutator;
}

impl Mutate<FuzzCase> for FuzzCaseMutator {
    fn mutate(
        &mut self,
        candidates: &mut Candidates<'_>,
        case: &mut FuzzCase,
    ) -> MutatisResult<()> {
        if candidates.shrink() {
            return shrink_case(candidates, case);
        }

        if !candidates.shrink() && case.ops.len() < MAX_STEPS {
            candidates.mutation(|context| {
                let index = context.rng().gen_index(case.ops.len() + 1).unwrap();
                let mut op_mutator = mutatis::mutators::default::<Op>();
                let op = op_mutator.generate(context)?;
                case.ops.insert(index, op);
                Ok(())
            })?;
        }

        if !case.ops.is_empty() {
            candidates.mutation(|context| {
                let index = context.rng().gen_index(case.ops.len()).unwrap();
                case.ops.remove(index);
                Ok(())
            })?;
        }

        if case.ops.len() >= 2 {
            candidates.mutation(|context| {
                let left = context.rng().gen_index(case.ops.len()).unwrap();
                let right = context.rng().gen_index(case.ops.len()).unwrap();
                case.ops.swap(left, right);
                Ok(())
            })?;
        }

        let mut op_mutator = mutatis::mutators::default::<Op>();
        for op in &mut case.ops {
            op_mutator.mutate(candidates, op)?;
        }

        Ok(())
    }
}

fn shrink_case(candidates: &mut Candidates<'_>, case: &mut FuzzCase) -> MutatisResult<()> {
    let len = case.ops.len();

    if len > 1 {
        candidates.mutation(|context| {
            random_multistep_shrink_case(case, context.rng());
            Ok(())
        })?;

        candidates.mutation_group((len - 1) as u32, |_context, which| {
            case.ops.truncate(which as usize + 1);
            Ok(())
        })?;

        let chunk_sizes = chunk_delete_sizes(len);
        let delete_count = chunk_sizes
            .iter()
            .map(|size| len.saturating_sub(*size) + 1)
            .sum::<usize>();
        candidates.mutation_group(delete_count as u32, |_context, mut which| {
            for size in chunk_sizes {
                let starts = len - size + 1;
                if which < starts as u32 {
                    let start = which as usize;
                    case.ops.drain(start..start + size);
                    return Ok(());
                }
                which -= starts as u32;
            }
            Ok(())
        })?;
    }

    for index in 0..len {
        let replacements = simplified_ops(&case.ops[index]);
        if replacements.is_empty() {
            continue;
        }

        candidates.mutation_group(replacements.len() as u32, |_context, which| {
            case.ops[index] = replacements[which as usize].clone();
            Ok(())
        })?;
    }

    let mut op_mutator = mutatis::mutators::default::<Op>();
    for op in &mut case.ops {
        op_mutator.mutate(candidates, op)?;
    }

    Ok(())
}

fn chunk_delete_sizes(len: usize) -> Vec<usize> {
    let mut sizes = Vec::new();
    let mut size = len / 2;
    while size > 1 {
        if !sizes.contains(&size) {
            sizes.push(size);
        }
        size /= 2;
    }
    sizes.push(1);
    sizes
}

#[derive(Clone, Debug, PartialEq)]
pub struct FuzzFailure {
    step: usize,
    op: String,
    message: String,
}

impl FuzzFailure {
    pub fn step(&self) -> usize {
        self.step
    }

    pub fn op(&self) -> &str {
        &self.op
    }

    pub fn message(&self) -> &str {
        &self.message
    }
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

pub fn decode_case(data: &[u8]) -> Option<FuzzCase> {
    let mut case = postcard::from_bytes::<FuzzCase>(data).ok()?;
    case.normalize();
    Some(case)
}

pub fn encode_case(case: &FuzzCase, data: &mut [u8], max_size: usize) -> Option<usize> {
    let size = max_size.min(data.len());
    let encoded = postcard::to_slice(case, &mut data[..size]).ok()?;
    Some(encoded.len())
}

pub fn encode_case_vec(case: &FuzzCase) -> Option<Vec<u8>> {
    postcard::to_allocvec(case).ok()
}

pub fn run_case(case: &FuzzCase) -> Result<(), FuzzFailure> {
    let mut state = Harness::fresh();
    for (step, op) in case.ops.iter().enumerate() {
        apply_step(&mut state, op).map_err(|message| FuzzFailure {
            step,
            op: format!("{op:?}"),
            message,
        })?;
    }
    Ok(())
}

pub fn print_case_trace(case: &FuzzCase, failure: &FuzzFailure) {
    print_ssr_diff_trace(&case.ops, failure.step, &failure.message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_case_roundtrips_and_replays() {
        let case = FuzzCase::seed();
        let mut bytes = [0; 4096];
        let size = encode_case(&case, &mut bytes, 4096).unwrap();
        let decoded = decode_case(&bytes[..size]).unwrap();
        assert_eq!(case, decoded);
        run_case(&decoded).unwrap();
    }
}
