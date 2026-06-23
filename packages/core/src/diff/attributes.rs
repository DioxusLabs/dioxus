//! Diffing for dynamic attributes.
//!
//! Templates keep static attributes in the packed op tape and store runtime attributes in
//! `VNode::dynamic_attrs`. Each dynamic attribute path points at the element that owns the
//! corresponding dynamic attribute slot. Several adjacent slots may point at the same element when
//! RSX mixes named dynamic attributes and spreads.
//!
//! Creating a template can write those slots in order because later writes naturally overwrite
//! earlier writes on the real element. Diffing needs a little more context: removing a later spread
//! can reveal an earlier dynamic attribute with the same key, or the static template attribute that
//! was loaded with the template. To preserve those "last write wins" semantics, the diff:
//!
//! 1. collects the dynamic attribute slots for the same element anchor;
//! 2. flattens the old and new slots for that element;
//! 3. reduces each side to the effective attribute for each `(name, namespace)` key, keeping the
//!    last matching attribute; and
//! 4. merges the old and new effective attribute lists to emit additions, updates, removals, and
//!    static-template restores.

use core::{cmp::Ordering, iter::Peekable};

use crate::innerlude::MountId;
use crate::{
    Attribute, AttributeValue, VNode, VirtualDom, WriteMutations, arena::MountedElementId,
    diff::template::DynamicAnchor, mutations::TargetedLazyScope,
};

/// Attribute identity as seen by renderers. Value changes do not affect the key, but namespace
/// changes do.
type AttributeKey = (&'static str, Option<&'static str>);

/// Reusable scratch for the two k-way merges in `diff_attribute_list`. Allocated once per
/// `diff_attributes` call and cleared on every merge.
#[derive(Default)]
pub(crate) struct AttributeDiffScratch<'a> {
    old_ranges: Vec<&'a [Attribute]>,
    old_offsets: Vec<usize>,
    new_ranges: Vec<&'a [Attribute]>,
    new_offsets: Vec<usize>,
}

impl VNode {
    /// Diff all dynamic attributes that can affect one mounted element.
    ///
    /// `from` and `to_attrs` are the flattened dynamic slots for the same template cursor. They may
    /// contain duplicate keys from multiple spreads or from a spread overriding a named attribute.
    /// Before we compare sides, each side is reduced to its effective, last-written attribute per
    /// key.
    pub(super) fn diff_attribute_list<'a>(
        &'a self,
        old_anchor: DynamicAnchor<'a>,
        new_anchor: DynamicAnchor<'a>,
        id: MountedElementId,
        mount: MountId,
        scratch: &mut AttributeDiffScratch<'a>,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) {
        let AttributeDiffScratch {
            old_ranges,
            old_offsets,
            new_ranges,
            new_offsets,
        } = scratch;
        let mut from_iter = iter_sorted_last_wins(
            old_anchor.attrs().map(|slot| slot.attrs()),
            old_ranges,
            old_offsets,
        )
        .peekable();
        let mut to_iter = iter_sorted_last_wins(
            new_anchor.attrs().map(|slot| slot.attrs()),
            new_ranges,
            new_offsets,
        )
        .peekable();

        let element_id = id.element_id();
        // The attribute diff never changes the active render target, so the targeted gate is
        // always satisfied here - equivalent to an unconditional lazy push.
        let mut to =
            TargetedLazyScope::new(to, dom.runtime.clone(), move |to| to.push_id(element_id));
        while let Some((key, old, new)) = Self::next_attribute_diff(&mut from_iter, &mut to_iter) {
            self.diff_dynamic_attribute(old_anchor, key, id, mount, old, new, dom, &mut to);
        }
    }

    /// Merge two sorted streams of effective attributes.
    ///
    /// Each returned item contains the key plus the old and/or new attribute for that key.
    fn next_attribute_diff<'a>(
        from_iter: &mut Peekable<impl Iterator<Item = &'a Attribute>>,
        to_iter: &mut Peekable<impl Iterator<Item = &'a Attribute>>,
    ) -> Option<(AttributeKey, Option<&'a Attribute>, Option<&'a Attribute>)> {
        match (from_iter.peek().copied(), to_iter.peek().copied()) {
            (Some(from), Some(to_attr)) => {
                let from_key = Self::attribute_key(from);
                let to_key = Self::attribute_key(to_attr);
                match from_key.cmp(&to_key) {
                    Ordering::Less => {
                        from_iter.next();
                        Some((from_key, Some(from), None))
                    }
                    Ordering::Greater => {
                        to_iter.next();
                        Some((to_key, None, Some(to_attr)))
                    }
                    Ordering::Equal => {
                        from_iter.next();
                        to_iter.next();
                        Some((to_key, Some(from), Some(to_attr)))
                    }
                }
            }
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

    fn diff_dynamic_attribute(
        &self,
        anchor: DynamicAnchor<'_>,
        key: AttributeKey,
        id: MountedElementId,
        mount: MountId,
        old: Option<&Attribute>,
        new: Option<&Attribute>,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) {
        let old_listener = matches!(old.map(|a| &a.value), Some(AttributeValue::Listener(_)));
        let new_listener = matches!(new.map(|a| &a.value), Some(AttributeValue::Listener(_)));

        // Listener-to-listener: events dispatch by path and the handler in the vdom is already current.
        if old_listener && new_listener {
            return;
        }

        let value_changed = old.map(|a| &a.value) != new.map(|a| &a.value);
        let volatile = old.is_some_and(|a| a.volatile) || new.is_some_and(|a| a.volatile);
        // If the value didn't change and neither side is volatile, then there's no need to update the attribute.
        if !value_changed && !volatile {
            return;
        }

        // Clear the old slot when the upcoming write won't naturally overwrite it: listeners
        // are torn down explicitly, and installing a listener doesn't clear a prior attribute.
        match (old_listener, new_listener, old) {
            // The old attribute was a listener and the new one is not, so remove the old listener.
            (true, _, Some(old)) => {
                to.remove_event_listener(&old.name[2..]);
            }
            // The old attribute was a value and the new one is a listener, so clear the old value that the new listener won't overwrite.
            (false, true, Some(_)) => {
                let (name, namespace) = key;
                to.set_attribute(name, namespace, &AttributeValue::None);
            }
            _ => {}
        }

        // Write the new value, restore the static template attribute, or clear the DOM attribute.
        // A removed listener has nothing attribute-shaped left to clear.
        if let Some(new) = new {
            Self::write_attribute_to_current(new, id, mount, dom, to);
        } else if !old_listener {
            Self::remove_attribute_or_restore_static(anchor, key, to)
        }
    }

    /// Get the identity key for an attribute
    fn attribute_key(attribute: &Attribute) -> AttributeKey {
        (attribute.name, attribute.namespace)
    }

    /// Restore the static template attribute that was shadowed by a dynamic attribute or clear the attribute.
    ///
    /// This is needed when an attribute from a spread disappears. The template load already wrote
    /// the static value during creation, but the dynamic attribute may have overwritten or removed
    /// it on a previous render.
    /// ```rust,ignore
    /// div { width: "15px", ..spread } // spread = [attribute("width", "25px")]
    /// ```
    /// Diffs to:
    /// ```rust,ignore
    /// div { width: "15px", ..spread } // spread = []
    /// ```
    fn remove_attribute_or_restore_static(
        anchor: DynamicAnchor<'_>,
        key: AttributeKey,
        to: &mut dyn WriteMutations,
    ) {
        let (name, namespace) = key;
        let value = anchor
            .static_attr_value_for_key(key)
            .map(|value| AttributeValue::Text(value.to_string()))
            .unwrap_or(AttributeValue::None);
        to.set_attribute(name, namespace, &value);
    }

    /// Write one dynamic attribute to the current renderer stack element.
    ///
    /// The caller must have already pushed `id` onto `to`.
    /// Listener attributes also need a `MountRef` in the runtime so event dispatch can find
    /// the VNode that owns the handler.
    pub(super) fn write_attribute_to_current(
        attribute: &Attribute,
        id: MountedElementId,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut dyn WriteMutations,
    ) {
        match &attribute.value {
            AttributeValue::Listener(_) => {
                dom.set_element_ref_for_mount(mount, id);
                to.add_event_listener(&attribute.name[2..]);
            }
            _ => {
                to.set_attribute(attribute.name, attribute.namespace, &attribute.value);
            }
        }
    }
}

/// K-way merge over attribute slots that are each individually sorted by their key.
///
/// Every dynamic attribute slot is required to be sorted by `(name, namespace)`:
/// - named attributes occupy a slot of length 1 (trivially sorted), and
/// - spread attributes are normalized by `DynamicValues::normalize` before they are stored on a
///   `VNode`.
///
/// Duplicate keys across or within slots collapse to the last occurrence in iteration order,
/// which matches the "later write wins" semantics of RSX source order.
fn iter_sorted_last_wins<'items, 'scratch>(
    slots: impl IntoIterator<Item = &'items [Attribute]>,
    ranges: &'scratch mut Vec<&'items [Attribute]>,
    offsets: &'scratch mut Vec<usize>,
) -> SortedRangeIter<'items, 'scratch> {
    ranges.clear();
    ranges.extend(slots);
    offsets.clear();
    offsets.resize(ranges.len(), 0);
    SortedRangeIter { ranges, offsets }
}

struct SortedRangeIter<'items, 'scratch> {
    ranges: &'scratch Vec<&'items [Attribute]>,
    offsets: &'scratch mut Vec<usize>,
}

impl<'items> Iterator for SortedRangeIter<'items, '_> {
    type Item = &'items Attribute;

    fn next(&mut self) -> Option<Self::Item> {
        let mut min_key = None;

        // Find the smallest key currently visible across every range.
        for (range, offset) in self.ranges.iter().zip(self.offsets.iter()) {
            if let Some(item) = range.get(*offset) {
                let item_key = VNode::attribute_key(item);
                match min_key {
                    None => min_key = Some(item_key),
                    Some(current_min_key) if item_key < current_min_key => {
                        min_key = Some(item_key);
                    }
                    Some(_) => {}
                }
            }
        }

        let min_key = min_key?;
        let mut last = None;

        // Drain that key from every matching range. Later ranges come later in RSX source order,
        // so the final item we see is the effective last-write-wins value.
        for (range_idx, range) in self.ranges.iter().enumerate() {
            while let Some(item) = range.get(self.offsets[range_idx]) {
                if VNode::attribute_key(item) != min_key {
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
fn test_iter_sorted_last_wins() {
    fn attr(name: &'static str, value: &'static str) -> Attribute {
        Attribute {
            name,
            value: AttributeValue::Text(value.to_string()),
            namespace: None,
            volatile: false,
        }
    }

    // Two sorted slots that share keys. The slot listed second wins on duplicates.
    let slot_a = [attr("a", "0"), attr("b", "1"), attr("c", "2")];
    let slot_b = [attr("a", "5"), attr("b", "3"), attr("d", "4")];
    let mut ranges = Vec::new();
    let mut offsets = Vec::new();
    let mut iter = iter_sorted_last_wins(
        [slot_a.as_slice(), slot_b.as_slice()],
        &mut ranges,
        &mut offsets,
    );
    assert_eq!(*iter.next().unwrap(), attr("a", "5"));
    assert_eq!(*iter.next().unwrap(), attr("b", "3"));
    assert_eq!(*iter.next().unwrap(), attr("c", "2"));
    assert_eq!(*iter.next().unwrap(), attr("d", "4"));
    assert!(iter.next().is_none());
}
