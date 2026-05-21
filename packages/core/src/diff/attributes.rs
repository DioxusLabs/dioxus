use core::iter::Peekable;
use std::cmp::Ordering;

use crate::{Attribute, VNode};

fn non_decreasing_run<I, F>(iter: &mut Peekable<I>, mut predicate: F) -> usize
where
    I: Iterator<Item: Copy>,
    F: FnMut(I::Item, I::Item) -> Ordering,
{
    let mut last: Option<<I as Iterator>::Item> = None;
    std::iter::from_fn(move || {
        iter.next_if(|item| {
            let non_decreasing = last.as_mut().is_none_or(|last| {
                matches!(predicate(*last, *item), Ordering::Less | Ordering::Equal)
            });
            last = Some(*item);
            non_decreasing
        })
    })
    .count()
}

/// A list of attribute groups split into sorted ranges.
struct SortedRanges<'a, T> {
    ranges: Box<[&'a [T]]>,
}

impl<'a, T> SortedRanges<'a, T> {
    fn new(attributes: &'a [T], sort_by: impl Fn(&T, &T) -> Ordering + Copy) -> Self {
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

    fn iter_sorted_last_wins(
        &self,
        sort_by: impl Fn(&T, &T) -> Ordering + Copy,
    ) -> impl Iterator<Item = &T> {
        let mut iters = self
            .ranges
            .iter()
            .map(|range| range.iter().peekable())
            .collect::<Box<[_]>>();

        // Generate items
        std::iter::from_fn(move || {
            // The current min iterators
            let mut min = Vec::new();
            let mut min_value = None;

            // Go through every iterator and their next value
            for (item, iter) in iters
                .iter_mut()
                // Only keep iterators that have a next value
                .filter_map(|iter| iter.peek().copied().map(|item| (item, iter)))
            {
                match min_value
                    .as_mut()
                    .map(|min_value| sort_by(item, *min_value))
                {
                    // If this item is less than the current min, clear the min list and add this iterator
                    Some(Ordering::Less) => {
                        min.clear();
                        min.push(iter);
                        min_value = Some(item);
                    }
                    // Otherwise if this item is equal to the current min, add this iterator to the min list so it gets drained as well
                    Some(Ordering::Equal) => min.push(iter),
                    _ => {}
                }
            }
            // Drain all the min iterators and return the last item (the one from the last range) so that it wins over the others
            min.iter_mut().filter_map(|iter| iter.next()).last()
        })
    }
}

#[test]
fn test_non_decreasing_run() {
    let mut iter = [1, 2, 3, 2, 4, 4].iter().peekable();
    assert_eq!(non_decreasing_run(&mut iter, |a, b| a.cmp(b)), 3);
    assert_eq!(non_decreasing_run(&mut iter, |a, b| a.cmp(b)), 1);
    assert_eq!(non_decreasing_run(&mut iter, |a, b| a.cmp(b)), 2);
}

#[test]
fn test_sorted_ranges() {
    let runs = [1, 2, 3, 2, 4, 1, 1];
    let sorted = SortedRanges::new(&runs, |a, b| a.cmp(b));
    println!("{:?}", sorted.ranges);
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
    println!("{:?}", sorted.ranges);
    let mut iter = sorted.iter_sorted_last_wins(Item::cmp);
    assert_eq!(*iter.next().unwrap(), Item { value: 1, id: 6 });
    assert_eq!(*iter.next().unwrap(), Item { value: 2, id: 3 });
    assert_eq!(*iter.next().unwrap(), Item { value: 3, id: 2 });
    assert_eq!(*iter.next().unwrap(), Item { value: 4, id: 4 });
    assert!(iter.next().is_none());
}

impl VNode {
    pub(crate) fn diff_attribute_list(
        &self,
        from: &[Attribute],
        to: &[Attribute],
        // ...
    ) {
        let sort_by = |a: &Attribute, b: &Attribute| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.namespace.cmp(&b.namespace))
        };
        let sorted_from = SortedRanges::new(from, sort_by);
        let sorted_to = SortedRanges::new(to, sort_by);

        let mut from_iter = sorted_from.iter_sorted_last_wins(sort_by).peekable();
        let mut to_iter = sorted_to.iter_sorted_last_wins(sort_by).peekable();

        loop {
            match (from_iter.peek(), to_iter.peek()) {
                (Some(from), Some(to)) => match sort_by(from, to) {
                    Ordering::Less => {
                        // from is less than to, so it was removed
                        println!("Removed attribute: {:?}", from);
                        from_iter.next();
                    }
                    Ordering::Greater => {
                        // to is less than from, so it was added
                        println!("Added attribute: {:?}", to);
                        to_iter.next();
                    }
                    Ordering::Equal => {
                        // from and to are equal, so they are unchanged
                        println!("Unchanged attribute: {:?}", from);
                        from_iter.next();
                        to_iter.next();
                    }
                },
                (Some(from), None) => {
                    // No more attributes in to, so the rest of from were removed
                    println!("Removed attribute: {:?}", from);
                    from_iter.next();
                }
                (None, Some(to)) => {
                    // No more attributes in from, so the rest of to were added
                    println!("Added attribute: {:?}", to);
                    to_iter.next();
                }
                (None, None) => break,
            }
        }
    }
}
