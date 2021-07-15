use std::collections::HashMap;

use crate::FC;

pub(crate) struct HeuristicsEngine {
    heuristics: HashMap<FcSlot, Heuristic>,
}

pub(crate) type FcSlot = *const ();

pub(crate) struct Heuristic {
    hooks: u32,
    bump_size: u64,
}

impl HeuristicsEngine {
    pub(crate) fn new() -> Self {
        Self {
            heuristics: HashMap::new(),
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
