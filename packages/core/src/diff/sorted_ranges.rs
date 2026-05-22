use core::cmp::Ordering;

/// Consume one non-decreasing run from a peekable iterator.
///
/// The first item that would make the run decrease starts the next range.
fn non_decreasing_run<T, F>(items: &[T], mut predicate: F) -> usize
where
    F: FnMut(&T, &T) -> Ordering,
{
    if items.is_empty() {
        return 0;
    }

    let mut len = 1;
    while let Some(next) = items.get(len) {
        if matches!(predicate(&items[len - 1], next), Ordering::Greater) {
            break;
        }
        len += 1;
    }
    len
}

/// A flattened attribute list split into locally sorted ranges.
///
/// Named dynamic attributes and well-formed spreads are usually already sorted by key, but
/// concatenating those chunks can still make the whole list unsorted. This helper finds the sorted
/// runs and lazily merges them instead of allocating and sorting a second copy of the attribute
/// list. Splitting at decreases also tolerates runtime spreads that are only partially sorted.
pub(super) struct SortedRanges<'items, 'scratch, T> {
    ranges: &'scratch [&'items [T]],
}

impl<'items, 'scratch, T> SortedRanges<'items, 'scratch, T> {
    pub(super) fn new(
        attribute_slots: impl IntoIterator<Item = &'items [T]>,
        ranges: &'scratch mut Vec<&'items [T]>,
        sort_by: impl Fn(&T, &T) -> Ordering + Copy,
    ) -> Self {
        ranges.clear();

        for mut remaining in attribute_slots {
            while !remaining.is_empty() {
                let run = non_decreasing_run(remaining, sort_by);
                let (run, rest) = remaining.split_at(run);
                ranges.push(run);
                remaining = rest;
            }
        }

        Self {
            ranges: ranges.as_slice(),
        }
    }

    pub(super) fn iter_sorted_last_wins<'iter, F>(
        &'iter self,
        offsets: &'iter mut Vec<usize>,
        sort_by: F,
    ) -> SortedRangeIter<'items, 'iter, T, F>
    where
        F: Fn(&T, &T) -> Ordering + Copy,
    {
        offsets.clear();
        offsets.resize(self.ranges.len(), 0);

        SortedRangeIter {
            ranges: self.ranges,
            offsets,
            sort_by,
        }
    }
}

pub(super) struct SortedRangeIter<'items, 'scratch, T, F> {
    ranges: &'scratch [&'items [T]],
    offsets: &'scratch mut Vec<usize>,
    sort_by: F,
}

impl<'items, T, F> Iterator for SortedRangeIter<'items, '_, T, F>
where
    F: Fn(&T, &T) -> Ordering + Copy,
{
    type Item = &'items T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut min_value = None;

        // Find the smallest key currently visible across every range.
        for (range, offset) in self.ranges.iter().zip(self.offsets.iter()) {
            if let Some(item) = range.get(*offset) {
                match min_value.map(|min_value| (self.sort_by)(item, min_value)) {
                    None | Some(Ordering::Less) => min_value = Some(item),
                    Some(Ordering::Equal | Ordering::Greater) => {}
                }
            }
        }

        let min_value = min_value?;
        let mut last = None;

        // Drain that key from every matching range. Later ranges come later in RSX source order,
        // so the final item we see is the effective last-write-wins value.
        for (range_idx, range) in self.ranges.iter().enumerate() {
            while let Some(item) = range.get(self.offsets[range_idx]) {
                if !matches!((self.sort_by)(item, min_value), Ordering::Equal) {
                    break;
                }
                last = Some(item);
                self.offsets[range_idx] += 1;
            }
        }

        last
    }
}

#[test]
fn test_non_decreasing_run() {
    let data = [1, 2, 3, 2, 4, 4];
    assert_eq!(non_decreasing_run(&data, |a, b| a.cmp(b)), 3);
    assert_eq!(non_decreasing_run(&data[3..], |a, b| a.cmp(b)), 3);
    assert_eq!(non_decreasing_run(&[], |a: &i32, b| a.cmp(b)), 0);
}

#[test]
fn test_sorted_ranges() {
    let runs = [1, 2, 3, 2, 4, 1, 1];
    let mut ranges = Vec::new();
    let sorted = SortedRanges::new([runs.as_slice()], &mut ranges, |a, b| a.cmp(b));
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
    let mut ranges = Vec::new();
    let mut offsets = Vec::new();
    let sorted = SortedRanges::new([runs.as_slice()], &mut ranges, Item::cmp);
    let mut iter = sorted.iter_sorted_last_wins(&mut offsets, Item::cmp);
    assert_eq!(*iter.next().unwrap(), Item { value: 1, id: 6 });
    assert_eq!(*iter.next().unwrap(), Item { value: 2, id: 3 });
    assert_eq!(*iter.next().unwrap(), Item { value: 3, id: 2 });
    assert_eq!(*iter.next().unwrap(), Item { value: 4, id: 4 });
    assert!(iter.next().is_none());
}
