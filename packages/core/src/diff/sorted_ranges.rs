use core::cmp::Ordering;

/// A k-way merge view over a set of attribute slots that are each individually sorted by key.
///
/// Every dynamic attribute slot is required to be sorted by `(name, namespace)`:
/// - named attributes occupy a slot of length 1 (trivially sorted), and
/// - spread attributes are user-provided lists that the rsx macro routes through
///   `dioxus_core::internal::debug_check_spread_sorted` to surface violations in debug builds.
///
/// This type assumes that invariant and only merges across slots.
pub(super) struct SortedRanges<'items, 'scratch, T> {
    ranges: &'scratch [&'items [T]],
}

impl<'items, 'scratch, T> SortedRanges<'items, 'scratch, T> {
    pub(super) fn new(
        attribute_slots: impl IntoIterator<Item = &'items [T]>,
        ranges: &'scratch mut Vec<&'items [T]>,
    ) -> Self {
        ranges.clear();
        ranges.extend(attribute_slots);
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
    // Two sorted slots that share a key. The slot listed second is the override winner.
    let slot_a = [
        Item { value: 1, id: 0 },
        Item { value: 2, id: 1 },
        Item { value: 3, id: 2 },
    ];
    let slot_b = [
        Item { value: 1, id: 5 },
        Item { value: 2, id: 3 },
        Item { value: 4, id: 4 },
    ];
    let mut ranges = Vec::new();
    let mut offsets = Vec::new();
    let sorted = SortedRanges::new([slot_a.as_slice(), slot_b.as_slice()], &mut ranges);
    let mut iter = sorted.iter_sorted_last_wins(&mut offsets, Item::cmp);
    assert_eq!(*iter.next().unwrap(), Item { value: 1, id: 5 });
    assert_eq!(*iter.next().unwrap(), Item { value: 2, id: 3 });
    assert_eq!(*iter.next().unwrap(), Item { value: 3, id: 2 });
    assert_eq!(*iter.next().unwrap(), Item { value: 4, id: 4 });
    assert!(iter.next().is_none());
}
