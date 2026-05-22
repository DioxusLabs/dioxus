use core::{cmp::Ordering, iter::Peekable};

/// Consume one non-decreasing run from a peekable iterator.
///
/// The first item that would make the run decrease is left in the iterator so the next call can
/// start a new range at that item.
fn non_decreasing_run<I, F>(iter: &mut Peekable<I>, mut predicate: F) -> usize
where
    I: Iterator<Item: Copy>,
    F: FnMut(I::Item, I::Item) -> Ordering,
{
    let mut last: Option<<I as Iterator>::Item> = None;
    std::iter::from_fn(move || {
        iter.next_if(|item| {
            let non_decreasing = last
                .as_ref()
                .is_none_or(|last| !matches!(predicate(*last, *item), Ordering::Greater));
            last = Some(*item);
            non_decreasing
        })
    })
    .count()
}

/// A flattened attribute list split into locally sorted ranges.
///
/// Named dynamic attributes and well-formed spreads are usually already sorted by key, but
/// concatenating those chunks can still make the whole list unsorted. This helper finds the sorted
/// runs and lazily merges them instead of allocating and sorting a second copy of the attribute
/// list. Splitting at decreases also tolerates runtime spreads that are only partially sorted.
pub(super) struct SortedRanges<'a, T> {
    ranges: Box<[&'a [T]]>,
}

impl<'a, T> SortedRanges<'a, T> {
    pub(super) fn new(attributes: &'a [T], sort_by: impl Fn(&T, &T) -> Ordering + Copy) -> Self {
        let mut iter = attributes.iter().peekable();
        let mut remaining = attributes;
        let mut ranges = Vec::new();

        loop {
            let run = non_decreasing_run(&mut iter, sort_by);
            let (run, rest) = remaining.split_at(run);
            if run.is_empty() {
                break;
            }
            ranges.push(run);
            remaining = rest;
        }

        Self {
            ranges: ranges.into_boxed_slice(),
        }
    }

    pub(super) fn iter_sorted_last_wins(
        &'a self,
        sort_by: impl Fn(&T, &T) -> Ordering + Copy + 'a,
    ) -> impl Iterator<Item = &'a T> + 'a {
        let mut iters = self
            .ranges
            .iter()
            .map(|range| range.iter().peekable())
            .collect::<Box<[_]>>();

        std::iter::from_fn(move || {
            let mut min = Vec::new();
            let mut min_value = None;

            // Find every range currently pointing at the smallest key. Equal keys must be drained
            // together so duplicate attributes collapse into one effective value.
            for (item, iter) in iters
                .iter_mut()
                .filter_map(|iter| iter.peek().copied().map(|item| (item, iter)))
            {
                match min_value.map(|min_value| sort_by(item, min_value)) {
                    None | Some(Ordering::Less) => {
                        min.clear();
                        min.push(iter);
                        min_value = Some(item);
                    }
                    Some(Ordering::Equal) => min.push(iter),
                    Some(Ordering::Greater) => {}
                }
            }

            let min_value = min_value?;
            // Drain all attributes with this key from the matching ranges. The last attribute in
            // RSX source order is the one that would have been written last during creation, so it
            // is the only value the rest of the diff should see.
            min.into_iter()
                .flat_map(|iter| {
                    std::iter::from_fn(|| {
                        iter.next_if(|item| matches!(sort_by(*item, min_value), Ordering::Equal))
                    })
                })
                .last()
        })
    }
}

#[test]
fn test_non_decreasing_run() {
    let mut iter = [1, 2, 3, 2, 4, 4].iter().peekable();
    assert_eq!(non_decreasing_run(&mut iter, |a, b| a.cmp(b)), 3);
    assert_eq!(non_decreasing_run(&mut iter, |a, b| a.cmp(b)), 3);
    assert_eq!(non_decreasing_run(&mut iter, |a, b| a.cmp(b)), 0);
}

#[test]
fn test_sorted_ranges() {
    let runs = [1, 2, 3, 2, 4, 1, 1];
    let sorted = SortedRanges::new(&runs, |a, b| a.cmp(b));
    assert_eq!(sorted.ranges.len(), 3);
    assert_eq!(sorted.ranges[0], &[runs[0], runs[1], runs[2]]);
    assert_eq!(sorted.ranges[1], &[runs[3], runs[4]]);
    assert_eq!(sorted.ranges[2], &[runs[5], runs[6]]);
}

#[test]
fn test_sorted_ranges_iter() {
    #[derive(Debug, PartialEq)]
    struct Item {
        value: i32,
        id: usize,
    }
    impl Item {
        fn cmp(&self, other: &Self) -> Ordering {
            self.value.cmp(&other.value)
        }
    }
    let runs = [
        Item { value: 1, id: 0 },
        Item { value: 2, id: 1 },
        Item { value: 3, id: 2 },
        Item { value: 2, id: 3 },
        Item { value: 4, id: 4 },
        Item { value: 1, id: 5 },
        Item { value: 1, id: 6 },
    ];
    let sorted = SortedRanges::new(&runs, Item::cmp);
    let mut iter = sorted.iter_sorted_last_wins(Item::cmp);
    assert_eq!(*iter.next().unwrap(), Item { value: 1, id: 6 });
    assert_eq!(*iter.next().unwrap(), Item { value: 2, id: 3 });
    assert_eq!(*iter.next().unwrap(), Item { value: 3, id: 2 });
    assert_eq!(*iter.next().unwrap(), Item { value: 4, id: 4 });
    assert!(iter.next().is_none());
}
