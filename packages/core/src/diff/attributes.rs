//! Diffing for dynamic attributes.
//!
//! Templates keep static attributes in `TemplateNode::Element` and store runtime attributes in
//! `VNode::dynamic_attrs`. Each entry in `template.attr_paths()` points at the element that owns
//! the corresponding dynamic attribute slot. Several adjacent slots may point at the same element
//! when RSX mixes named dynamic attributes and spreads.
//!
//! Creating a template can write those slots in order because later writes naturally overwrite
//! earlier writes on the real element. Diffing needs a little more context: removing a later spread
//! can reveal an earlier dynamic attribute with the same key, or the static template attribute that
//! was loaded with the template. To preserve those "last write wins" semantics, the diff:
//!
//! 1. groups all adjacent dynamic attribute slots for the same element path;
//! 2. flattens the old and new slots for that element;
//! 3. reduces each side to the effective attribute for each `(name, namespace)` key, keeping the
//!    last matching attribute; and
//! 4. merges the old and new effective attribute lists to emit additions, updates, removals, and
//!    static-template fallbacks.

use core::{iter::Peekable, ops::Range};
use std::cmp::Ordering;

use crate::innerlude::MountId;
use crate::{
    Attribute, AttributeValue, TemplateAttribute, TemplateNode, VNode, VirtualDom, WriteMutations,
    arena::ElementId,
    innerlude::{ElementPath, ElementRef},
};

/// Attribute identity as seen by renderers. Value changes do not affect the key, but namespace
/// changes do.
type AttributeKey = (&'static str, Option<&'static str>);

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

impl VNode {
    pub(super) fn diff_attributes(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let mount_id = new.mount.get();
        let attr_paths = self.template.attr_paths();
        let mut idx = 0;

        while idx < attr_paths.len() {
            let path = attr_paths[idx];
            // Multiple dynamic attribute slots can target the same element. Diff them as a single
            // group so duplicate keys obey the same overwrite order they used during creation.
            let attr_group = self.dynamic_attribute_group_starting_at(idx);
            // Every slot in the group is mounted to the same real element, so the first slot's id
            // is enough for all mutations generated by this group.
            let attribute_id = dom.get_mounted_dyn_attr(mount_id, idx);
            let mut from = Vec::new();
            let mut to_attrs = Vec::new();

            for slot_idx in attr_group.clone() {
                from.extend(self.dynamic_attrs[slot_idx].iter());
                to_attrs.extend(new.dynamic_attrs[slot_idx].iter());
            }

            self.diff_attribute_list(path, attribute_id, mount_id, &from, &to_attrs, dom, to);

            idx = attr_group.end;
        }
    }

    /// Diff all dynamic attributes that can affect one mounted element.
    ///
    /// `from` and `to_attrs` are the flattened dynamic slots for the same template path. They may
    /// contain duplicate keys from multiple spreads or from a spread overriding a named attribute.
    /// Before we compare sides, each side is reduced to its effective, last-written attribute per
    /// key.
    fn diff_attribute_list(
        &self,
        path: &'static [u8],
        id: ElementId,
        mount: MountId,
        from: &[&Attribute],
        to_attrs: &[&Attribute],
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let sort_by = |a: &&Attribute, b: &&Attribute| Self::compare_attribute_keys(a, b);
        let sorted_from = SortedRanges::new(from, sort_by);
        let sorted_to = SortedRanges::new(to_attrs, sort_by);

        let mut from_iter = sorted_from
            .iter_sorted_last_wins(sort_by)
            .copied()
            .peekable();
        let mut to_iter = sorted_to.iter_sorted_last_wins(sort_by).copied().peekable();

        while let Some((key, old, new)) = Self::next_attribute_diff(&mut from_iter, &mut to_iter) {
            self.diff_dynamic_attribute(path, key, id, mount, old, new, dom, to);
        }
    }

    /// Merge two sorted streams of effective attributes.
    ///
    /// Each returned item contains the key plus the old and/or new attribute for that key. This is
    /// the same shape as a map diff, but it avoids building maps because the inputs are already
    /// emitted in sorted order.
    fn next_attribute_diff<'a>(
        from_iter: &mut Peekable<impl Iterator<Item = &'a Attribute>>,
        to_iter: &mut Peekable<impl Iterator<Item = &'a Attribute>>,
    ) -> Option<(AttributeKey, Option<&'a Attribute>, Option<&'a Attribute>)> {
        match (from_iter.peek().copied(), to_iter.peek().copied()) {
            (Some(from), Some(to_attr)) => match Self::compare_attribute_keys(from, to_attr) {
                Ordering::Less => {
                    from_iter.next();
                    Some((Self::attribute_key(from), Some(from), None))
                }
                Ordering::Greater => {
                    to_iter.next();
                    Some((Self::attribute_key(to_attr), None, Some(to_attr)))
                }
                Ordering::Equal => {
                    from_iter.next();
                    to_iter.next();
                    Some((Self::attribute_key(to_attr), Some(from), Some(to_attr)))
                }
            },
            (Some(from), None) => {
                from_iter.next();
                Some((Self::attribute_key(from), Some(from), None))
            }
            (None, Some(to_attr)) => {
                to_iter.next();
                Some((Self::attribute_key(to_attr), None, Some(to_attr)))
            }
            (None, None) => None,
        }
    }

    /// Return the contiguous run of dynamic attribute slots mounted to the same template path.
    ///
    /// Attribute paths are emitted in template order, so all slots for a single element are
    /// adjacent. Grouping them here is what lets the diff handle duplicate keys across spreads.
    fn dynamic_attribute_group_starting_at(&self, start: usize) -> Range<usize> {
        let attr_paths = self.template.attr_paths();
        let path = attr_paths[start];
        let mut end = start + 1;

        while end < attr_paths.len() && attr_paths[end] == path {
            end += 1;
        }

        start..end
    }

    fn compare_attribute_keys(left: &Attribute, right: &Attribute) -> Ordering {
        Self::attribute_key(left).cmp(&Self::attribute_key(right))
    }

    fn attribute_key(attribute: &Attribute) -> AttributeKey {
        (attribute.name, attribute.namespace)
    }

    fn attribute_value_changed(old: &Attribute, new: &Attribute) -> bool {
        match (&old.value, &new.value) {
            (AttributeValue::Text(left), AttributeValue::Text(right)) => left != right,
            (AttributeValue::Float(left), AttributeValue::Float(right)) => left != right,
            (AttributeValue::Int(left), AttributeValue::Int(right)) => left != right,
            (AttributeValue::Bool(left), AttributeValue::Bool(right)) => left != right,
            (AttributeValue::Any(left), AttributeValue::Any(right)) => {
                !left.as_ref().any_cmp(right.as_ref())
            }
            (AttributeValue::None, AttributeValue::None) => false,
            // Listener handler values are owned by the VNode and do not require renderer mutations
            // as long as the listener key remains present.
            (AttributeValue::Listener(_), AttributeValue::Listener(_)) => false,
            _ => true,
        }
    }

    /// Apply one effective attribute diff to the renderer.
    ///
    /// Event listeners have distinct create/remove mutations, so transitions between listener and
    /// non-listener values must first remove the old representation. For ordinary attributes,
    /// removing a dynamic value either restores the static template value it was shadowing or emits
    /// `AttributeValue::None` to remove the key entirely.
    fn diff_dynamic_attribute(
        &self,
        path: &'static [u8],
        key: AttributeKey,
        id: ElementId,
        mount: MountId,
        old: Option<&Attribute>,
        new: Option<&Attribute>,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        match (
            Self::attribute_is_listener(old),
            Self::attribute_is_listener(new),
        ) {
            (true, true) => {}
            (true, false) | (false, true) => {
                self.remove_dynamic_attribute(old, id, to);
                if let Some(new) = new {
                    self.write_attribute(path, new, id, mount, dom, to);
                } else {
                    self.write_static_attribute_fallback(path, key, id, to);
                }
            }
            (false, false) if Self::attribute_should_update(old, new) => {
                if let Some(new) = new {
                    self.write_attribute(path, new, id, mount, dom, to);
                } else {
                    self.write_static_attribute_fallback_or_remove(path, key, id, to);
                }
            }
            (false, false) => {}
        }
    }

    fn attribute_is_listener(attribute: Option<&Attribute>) -> bool {
        attribute.is_some_and(|attribute| matches!(&attribute.value, AttributeValue::Listener(_)))
    }

    fn attribute_should_update(old: Option<&Attribute>, new: Option<&Attribute>) -> bool {
        Self::attribute_volatile(old)
            || Self::attribute_volatile(new)
            || Self::dynamic_attribute_changed(old, new)
    }

    fn attribute_volatile(attribute: Option<&Attribute>) -> bool {
        attribute.is_some_and(|attribute| attribute.volatile)
    }

    fn dynamic_attribute_changed(old: Option<&Attribute>, new: Option<&Attribute>) -> bool {
        match (old, new) {
            (None, None) => false,
            (Some(left), Some(right)) => Self::attribute_value_changed(left, right),
            (None, Some(_)) | (Some(_), None) => true,
        }
    }

    /// Remove the old dynamic representation for a key.
    ///
    /// This is used before writing a replacement whose kind changed, such as `onclick` moving from
    /// an event listener to a normal attribute.
    fn remove_dynamic_attribute(
        &self,
        attribute: Option<&Attribute>,
        id: ElementId,
        to: &mut impl WriteMutations,
    ) {
        match attribute {
            None => {}
            Some(attribute) if matches!(&attribute.value, AttributeValue::Listener(_)) => {
                self.remove_event_listener(attribute, id, to);
            }
            Some(attribute) => {
                to.set_attribute(
                    attribute.name,
                    attribute.namespace,
                    &AttributeValue::None,
                    id,
                );
            }
        }
    }

    fn remove_event_listener(
        &self,
        attribute: &Attribute,
        id: ElementId,
        to: &mut impl WriteMutations,
    ) {
        to.remove_event_listener(&attribute.name[2..], id);
    }

    /// Restore the static template attribute for `key`, or remove the attribute if no static value
    /// exists under the dynamic slot.
    fn write_static_attribute_fallback_or_remove(
        &self,
        path: &'static [u8],
        key: AttributeKey,
        id: ElementId,
        to: &mut impl WriteMutations,
    ) {
        if !self.write_static_attribute_fallback(path, key, id, to) {
            to.set_attribute(key.0, key.1, &AttributeValue::None, id);
        }
    }

    /// Restore the static template attribute that was shadowed by a dynamic attribute.
    ///
    /// This is needed when an attribute from a spread disappears. The template load already wrote
    /// the static value during creation, but the dynamic attribute may have overwritten or removed
    /// it on a previous render.
    fn write_static_attribute_fallback(
        &self,
        path: &'static [u8],
        key: AttributeKey,
        id: ElementId,
        to: &mut impl WriteMutations,
    ) -> bool {
        if let Some(value) = self.static_template_attribute_value(path, key) {
            let value = AttributeValue::Text(value.to_string());
            to.set_attribute(key.0, key.1, &value, id);
            true
        } else {
            false
        }
    }

    fn static_template_attribute_value(
        &self,
        path: &'static [u8],
        key: AttributeKey,
    ) -> Option<&'static str> {
        let attrs = self.template_node_at_path(path).element_attrs();
        // Static attributes are stored first and sorted by name. Search only that prefix, then
        // filter by namespace because the ordering guarantee is by name.
        let start = attrs.partition_point(|attr| match attr {
            TemplateAttribute::Static { name, .. } => *name < key.0,
            TemplateAttribute::Dynamic { .. } => false,
        });

        attrs[start..]
            .iter()
            .take_while(
                |attr| matches!(attr, TemplateAttribute::Static { name, .. } if *name == key.0),
            )
            .filter_map(|attr| match attr {
                TemplateAttribute::Static {
                    value, namespace, ..
                } if *namespace == key.1 => Some(*value),
                _ => None,
            })
            .last()
    }

    /// Resolve the template element that owns a dynamic attribute path.
    fn template_node_at_path(&self, path: &'static [u8]) -> &'static TemplateNode {
        let (root_idx, child_path) = path
            .split_first()
            .expect("template attribute paths should not be empty");
        let mut node = &self.template.roots()[*root_idx as usize];

        for child_idx in child_path {
            node = node.element_child(*child_idx as usize);
        }

        node
    }

    /// Write one dynamic attribute to an already mounted element.
    ///
    /// Listener attributes also need an `ElementRef` in the runtime so event dispatch can find
    /// the VNode that owns the handler.
    pub(super) fn write_attribute(
        &self,
        path: &'static [u8],
        attribute: &Attribute,
        id: ElementId,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        match &attribute.value {
            AttributeValue::Listener(_) => {
                let element_ref = ElementRef {
                    path: ElementPath { path },
                    mount,
                };
                let mut elements = dom.runtime.elements.borrow_mut();
                elements[id.0] = Some(element_ref);
                to.create_event_listener(&attribute.name[2..], id);
            }
            _ => {
                to.set_attribute(attribute.name, attribute.namespace, &attribute.value, id);
            }
        }
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
