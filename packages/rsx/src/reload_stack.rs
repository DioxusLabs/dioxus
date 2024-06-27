/// A vec that's optimized for finding and removing elements that match a predicate.
///
/// Currently will do a linear search for the first element that matches the predicate.
/// Uses a reverse lookup so we pop elements off, shrinking the vec and making future lookups faster.
/// If you look things up in reverse order, and they match, this will be O(1)
///
/// The motivating factor here is that hashes are expensive and actually quite hard to maintain for
/// callbody. Hashing would imply a number of nested invariants that are hard to maintain.
///
/// Deriving hash will start to slurp up private fields which is not what we want, so the comparison
/// function is moved here to the reloadstack interface.
pub struct ReloadStack<T> {
    stack: Vec<Option<T>>,
}

impl<T> ReloadStack<T> {
    pub fn new(f: impl DoubleEndedIterator<Item = T>) -> Self {
        let stack = f.map(Some).collect();
        Self { stack }
    }

    pub fn remove(&mut self, idx: usize) -> Option<T> {
        self.stack.get_mut(idx).unwrap().take()
    }

    pub fn pop_where<F>(&mut self, f: F) -> Option<T>
    where
        F: Fn(&T) -> bool,
    {
        let idx = self
            .stack
            .iter()
            .position(|x| if let Some(x) = x { f(x) } else { false })?;

        self.remove(idx)
    }

    /// Returns the index and score of the highest scored element
    ///
    /// shortcircuits if the score is usize::MAX
    /// returns None if the score was 0
    pub fn highest_score(&self, score: impl Fn(&T) -> usize) -> Option<(usize, usize)> {
        let mut highest_score = 0;
        let mut best = None;

        for (idx, x) in self.stack.iter().enumerate() {
            if let Some(x) = x {
                let scored = score(x);
                if scored > highest_score {
                    best = Some(idx);
                    highest_score = scored;
                }

                if highest_score == usize::MAX {
                    break;
                }
            }
        }

        if highest_score == 0 {
            return None;
        }

        best.map(|idx| (idx, highest_score))
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn raw_len(&self) -> usize {
        self.stack.len()
    }
}

#[test]
fn searches_and_works() {
    let mut stack = ReloadStack::new(vec![1, 2, 3, 4, 5].into_iter());

    assert_eq!(stack.pop_where(|x| *x == 3), Some(3));
    assert_eq!(stack.pop_where(|x| *x == 1), Some(1));
    assert_eq!(stack.pop_where(|x| *x == 5), Some(5));
    assert_eq!(stack.pop_where(|x| *x == 2), Some(2));
    assert_eq!(stack.pop_where(|x| *x == 4), Some(4));
    assert_eq!(stack.pop_where(|x| *x == 4), None);

    assert!(stack.is_empty());
}
