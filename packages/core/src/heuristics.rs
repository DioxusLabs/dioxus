use std::collections::HashMap;

use fxhash::FxHashMap;

use crate::FC;

/// Provides heuristics to the "SharedResources" object for improving allocation performance.
///
/// This heuristics engine records the memory footprint of bump arenas and hook lists for each component. These records are
/// then used later on to optimize the initial allocation for future components. This helps save large allocations later on
/// that would slow down the diffing and initialization process.
///
///
pub struct HeuristicsEngine {
    heuristics: FxHashMap<FcSlot, Heuristic>,
}

pub type FcSlot = *const ();

pub struct Heuristic {
    hooks: u32,
    bump_size: u64,
}

impl HeuristicsEngine {
    pub(crate) fn new() -> Self {
        Self {
            heuristics: FxHashMap::default(),
        }
    }

    fn recommend<T>(&mut self, fc: FC<T>, heuristic: Heuristic) {
        let g = fc as FcSlot;
        let e = self.heuristics.entry(g);
    }

    fn get_recommendation<T>(&mut self, fc: FC<T>) -> &Heuristic {
        let id = fc as FcSlot;

        self.heuristics.entry(id).or_insert(Heuristic {
            bump_size: 100,
            hooks: 10,
        })
    }
}

#[test]
fn types_work() {
    let engine = HeuristicsEngine::new();
}
