use crate::{node::PreventDefault, TuiDom};

use dioxus_native_core::{
    tree::TreeView,
    utils::{ElementProduced, PersistantElementIter},
    RealNodeId,
};
use dioxus_native_core_macro::sorted_str_slice;

use std::{cmp::Ordering, num::NonZeroU16};

use dioxus_native_core::{
    node_ref::{AttributeMask, NodeMask, NodeView},
    state::NodeDepState,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum FocusLevel {
    #[default]
    Unfocusable,
    Focusable,
    Ordered(std::num::NonZeroU16),
}

impl FocusLevel {
    pub fn focusable(&self) -> bool {
        match self {
            FocusLevel::Unfocusable => false,
            FocusLevel::Focusable => true,
            FocusLevel::Ordered(_) => true,
        }
    }
}

impl PartialOrd for FocusLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (FocusLevel::Unfocusable, FocusLevel::Unfocusable) => Some(std::cmp::Ordering::Equal),
            (FocusLevel::Unfocusable, FocusLevel::Focusable) => Some(std::cmp::Ordering::Less),
            (FocusLevel::Unfocusable, FocusLevel::Ordered(_)) => Some(std::cmp::Ordering::Less),
            (FocusLevel::Focusable, FocusLevel::Unfocusable) => Some(std::cmp::Ordering::Greater),
            (FocusLevel::Focusable, FocusLevel::Focusable) => Some(std::cmp::Ordering::Equal),
            (FocusLevel::Focusable, FocusLevel::Ordered(_)) => Some(std::cmp::Ordering::Greater),
            (FocusLevel::Ordered(_), FocusLevel::Unfocusable) => Some(std::cmp::Ordering::Greater),
            (FocusLevel::Ordered(_), FocusLevel::Focusable) => Some(std::cmp::Ordering::Less),
            (FocusLevel::Ordered(a), FocusLevel::Ordered(b)) => a.partial_cmp(b),
        }
    }
}

impl Ord for FocusLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub(crate) struct Focus {
    pub level: FocusLevel,
}

impl NodeDepState for Focus {
    type DepState = ();
    type Ctx = ();
    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(FOCUS_ATTRIBUTES)).with_listeners();

    fn reduce(&mut self, node: NodeView<'_>, _sibling: (), _: &Self::Ctx) -> bool {
        let new = Focus {
            level: if let Some(a) = node
                .attributes()
                .and_then(|mut a| a.find(|a| a.attribute.name == "tabindex"))
            {
                if let Some(index) = a
                    .value
                    .as_int()
                    .or_else(|| a.value.as_text().and_then(|v| v.parse::<i64>().ok()))
                {
                    match index.cmp(&0) {
                        Ordering::Less => FocusLevel::Unfocusable,
                        Ordering::Equal => FocusLevel::Focusable,
                        Ordering::Greater => {
                            FocusLevel::Ordered(NonZeroU16::new(index as u16).unwrap())
                        }
                    }
                } else {
                    FocusLevel::Unfocusable
                }
            } else if node
                .listeners()
                .and_then(|mut listeners| {
                    listeners
                        .any(|l| FOCUS_EVENTS.binary_search(&l).is_ok())
                        .then_some(())
                })
                .is_some()
            {
                FocusLevel::Focusable
            } else {
                FocusLevel::Unfocusable
            },
        };
        if *self != new {
            *self = new;
            true
        } else {
            false
        }
    }
}

const FOCUS_EVENTS: &[&str] = &sorted_str_slice!(["keydown", "keypress", "keyup"]);
const FOCUS_ATTRIBUTES: &[&str] = &sorted_str_slice!(["tabindex"]);

#[derive(Default)]
pub(crate) struct FocusState {
    pub(crate) focus_iter: PersistantElementIter,
    pub(crate) last_focused_id: Option<RealNodeId>,
    pub(crate) focus_level: FocusLevel,
    pub(crate) dirty: bool,
}

impl FocusState {
    /// Returns true if the focus has changed.
    pub fn progress(&mut self, rdom: &mut TuiDom, forward: bool) -> bool {
        if let Some(last) = self.last_focused_id {
            if rdom[last].state.prevent_default == PreventDefault::KeyDown {
                return false;
            }
        }
        // the id that started focused to track when a loop has happened
        let mut loop_marker_id = self.last_focused_id;
        let focus_level = &mut self.focus_level;
        let mut next_focus = None;

        loop {
            let new = if forward {
                self.focus_iter.next(rdom)
            } else {
                self.focus_iter.prev(rdom)
            };
            let new_id = new.id();
            if let ElementProduced::Looped(_) = new {
                let mut closest_level = None;

                if forward {
                    // find the closest focusable element after the current level
                    rdom.traverse_depth_first(|n| {
                        let node_level = n.state.focus.level;
                        if node_level != *focus_level
                            && node_level.focusable()
                            && node_level > *focus_level
                        {
                            if let Some(level) = &mut closest_level {
                                if node_level < *level {
                                    *level = node_level;
                                }
                            } else {
                                closest_level = Some(node_level);
                            }
                        }
                    });
                } else {
                    // find the closest focusable element before the current level
                    rdom.traverse_depth_first(|n| {
                        let node_level = n.state.focus.level;
                        if node_level != *focus_level
                            && node_level.focusable()
                            && node_level < *focus_level
                        {
                            if let Some(level) = &mut closest_level {
                                if node_level > *level {
                                    *level = node_level;
                                }
                            } else {
                                closest_level = Some(node_level);
                            }
                        }
                    });
                }

                // extend the loop_marker_id to allow for another pass
                loop_marker_id = None;

                if let Some(level) = closest_level {
                    *focus_level = level;
                } else if forward {
                    *focus_level = FocusLevel::Unfocusable;
                } else {
                    *focus_level = FocusLevel::Focusable;
                }
            }

            // once we have looked at all the elements exit the loop
            if let Some(last) = loop_marker_id {
                if new_id == last {
                    break;
                }
            } else {
                loop_marker_id = Some(new_id);
            }

            let current_level = rdom[new_id].state.focus.level;
            let after_previous_focused = if forward {
                current_level >= *focus_level
            } else {
                current_level <= *focus_level
            };
            if after_previous_focused && current_level.focusable() && current_level == *focus_level
            {
                next_focus = Some(new_id);
                break;
            }
        }

        if let Some(id) = next_focus {
            if !rdom[id].state.focus.level.focusable() {
                panic!()
            }
            rdom[id].state.focused = true;
            if let Some(old) = self.last_focused_id.replace(id) {
                rdom[old].state.focused = false;
            }
            // reset the position to the currently focused element
            while self.focus_iter.next(rdom).id() != id {}
            self.dirty = true;
            return true;
        }

        false
    }

    pub(crate) fn prune(&mut self, mutations: &dioxus_core::Mutations, rdom: &TuiDom) {
        fn remove_children(
            to_prune: &mut [&mut Option<RealNodeId>],
            rdom: &TuiDom,
            removed: RealNodeId,
        ) {
            for opt in to_prune.iter_mut() {
                if let Some(id) = opt {
                    if *id == removed {
                        **opt = None;
                    }
                }
            }
            if let Some(children) = &rdom.children_ids(removed) {
                for child in *children {
                    remove_children(to_prune, rdom, *child);
                }
            }
        }
        if self.focus_iter.prune(mutations, rdom) {
            self.dirty = true;
        }
        for m in &mutations.edits {
            match m {
                dioxus_core::Mutation::ReplaceWith { id, .. } => remove_children(
                    &mut [&mut self.last_focused_id],
                    rdom,
                    rdom.element_to_node_id(*id),
                ),
                dioxus_core::Mutation::Remove { id } => remove_children(
                    &mut [&mut self.last_focused_id],
                    rdom,
                    rdom.element_to_node_id(*id),
                ),
                _ => (),
            }
        }
    }

    pub(crate) fn set_focus(&mut self, rdom: &mut TuiDom, id: RealNodeId) {
        if let Some(old) = self.last_focused_id.replace(id) {
            rdom[old].state.focused = false;
        }
        let state = &mut rdom[id].state;
        state.focused = true;
        self.focus_level = state.focus.level;
        // reset the position to the currently focused element
        while self.focus_iter.next(rdom).id() != id {}
        self.dirty = true;
    }

    pub(crate) fn clean(&mut self) -> bool {
        let old = self.dirty;
        self.dirty = false;
        old
    }
}
