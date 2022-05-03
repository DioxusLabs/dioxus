use crate::Dom;

use dioxus_core::ElementId;
use dioxus_native_core::utils::{ElementProduced, PersistantElementIter};

use std::num::NonZeroU16;

use dioxus_native_core::{
    node_ref::{AttributeMask, NodeMask, NodeView},
    real_dom::NodeType,
    state::NodeDepState,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Ord)]
pub(crate) enum FocusLevel {
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

impl Default for FocusLevel {
    fn default() -> Self {
        FocusLevel::Unfocusable
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub(crate) struct Focus {
    pub pass_focus: bool,
    pub level: FocusLevel,
}

impl NodeDepState for Focus {
    type Ctx = ();
    type DepState = ();
    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(FOCUS_ATTRIBUTES)).with_listeners();

    fn reduce(&mut self, node: NodeView<'_>, _sibling: &Self::DepState, _: &Self::Ctx) -> bool {
        let new = Focus {
            pass_focus: !node
                .attributes()
                .any(|a| a.name == "dioxus-prevent-default" && a.value.trim() == "true"),
            level: if let Some(a) = node.attributes().find(|a| a.name == "tabindex") {
                if let Ok(index) = a.value.parse::<i32>() {
                    if index < 0 {
                        FocusLevel::Unfocusable
                    } else if index == 0 {
                        FocusLevel::Focusable
                    } else {
                        FocusLevel::Ordered(NonZeroU16::new(index as u16).unwrap())
                    }
                } else {
                    FocusLevel::Unfocusable
                }
            } else {
                if node
                    .listeners()
                    .iter()
                    .any(|l| FOCUS_EVENTS.binary_search(&l.event).is_ok())
                {
                    FocusLevel::Focusable
                } else {
                    FocusLevel::Unfocusable
                }
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

// must be sorted
const FOCUS_EVENTS: &[&str] = &["keydown", "keypress", "keyup"];
const FOCUS_ATTRIBUTES: &[&str] = &["dioxus-prevent-default", "tabindex"];

#[derive(Default)]
pub(crate) struct FocusState {
    pub(crate) focus_iter: PersistantElementIter,
    pub(crate) last_focused_id: Option<ElementId>,
    pub(crate) focus_level: FocusLevel,
}

impl FocusState {
    // returns true if the focus has changed
    pub fn progress(&mut self, rdom: &mut Dom, forward: bool) -> bool {
        if let Some(last) = self.last_focused_id {
            if !rdom[last].state.focus.pass_focus {
                return false;
            }
        }
        let mut loop_marker_id = self.last_focused_id;
        let focus_level = &mut self.focus_level;
        let mut next_focus = None;
        let starting_focus_level = *focus_level;

        loop {
            let new = if forward {
                self.focus_iter.next(&rdom)
            } else {
                self.focus_iter.prev(&rdom)
            };
            let new_id = new.id();
            let current_level = rdom[new_id].state.focus.level;
            if let ElementProduced::Looped(_) = new {
                let mut closest_level = None;

                if forward {
                    // find the closest focusable element after the current level
                    rdom.traverse_depth_first(|n| {
                        let current_level = n.state.focus.level;
                        if current_level != *focus_level {
                            if current_level > *focus_level {
                                if let Some(level) = &mut closest_level {
                                    if current_level < *level {
                                        *level = current_level;
                                    }
                                } else {
                                    closest_level = Some(current_level);
                                }
                            }
                        }
                    });
                } else {
                    // find the closest focusable element before the current level
                    rdom.traverse_depth_first(|n| {
                        let current_level = n.state.focus.level;
                        if current_level != *focus_level {
                            if current_level < *focus_level {
                                if let Some(level) = &mut closest_level {
                                    if current_level > *level {
                                        *level = current_level;
                                    }
                                } else {
                                    closest_level = Some(current_level);
                                }
                            }
                        }
                    });
                }

                // extend the loop_marker_id to allow for another pass
                loop_marker_id = None;

                if let Some(level) = closest_level {
                    *focus_level = level;
                } else {
                    if forward {
                        *focus_level = FocusLevel::Unfocusable;
                    } else {
                        *focus_level = FocusLevel::Focusable;
                    }
                }

                // if the focus level looped, we are done
                if *focus_level == starting_focus_level {
                    break;
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

            let after_previous_focused = if forward {
                current_level >= *focus_level
            } else {
                current_level <= *focus_level
            };
            if after_previous_focused && current_level.focusable() {
                if current_level == *focus_level {
                    next_focus = Some((new_id, current_level));
                    break;
                }
            }
        }

        if let Some((id, order)) = next_focus {
            if order.focusable() {
                rdom[id].state.focused = true;
                if let Some(old) = self.last_focused_id.replace(id) {
                    rdom[old].state.focused = false;
                }
                // reset the position to the currently focused element
                while if forward {
                    self.focus_iter.next(&rdom).id()
                } else {
                    self.focus_iter.prev(&rdom).id()
                } != id
                {}
                return true;
            }
        }

        false
    }

    pub(crate) fn prune(&mut self, mutations: &dioxus_core::Mutations, rdom: &Dom) {
        fn remove_children(
            to_prune: &mut [&mut Option<ElementId>],
            rdom: &Dom,
            removed: ElementId,
        ) {
            for opt in to_prune.iter_mut() {
                if let Some(id) = opt {
                    if *id == removed {
                        **opt = None;
                    }
                }
            }
            if let NodeType::Element { children, .. } = &rdom[removed].node_type {
                for child in children {
                    remove_children(to_prune, rdom, *child);
                }
            }
        }
        for m in &mutations.edits {
            match m {
                dioxus_core::DomEdit::ReplaceWith { root, .. } => remove_children(
                    &mut [&mut self.last_focused_id],
                    rdom,
                    ElementId(*root as usize),
                ),
                dioxus_core::DomEdit::Remove { root } => remove_children(
                    &mut [&mut self.last_focused_id],
                    rdom,
                    ElementId(*root as usize),
                ),
                _ => (),
            }
        }
    }
}
