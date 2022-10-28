use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

// pub struct ElementId(pub NonZeroUsize);

// impl Default for ElementId {
//     fn default() -> Self {
//         Self(NonZeroUsize::new(1).unwrap())
//     }
// }

pub struct ElementArena {
    counter: usize,
}

impl Default for ElementArena {
    fn default() -> Self {
        Self { counter: 1 }
    }
}

impl ElementArena {
    pub fn next(&mut self) -> ElementId {
        let id = self.counter;
        self.counter += 1;
        ElementId(id)
    }
}
