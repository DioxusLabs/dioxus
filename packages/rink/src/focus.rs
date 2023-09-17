use crate::prevent_default::PreventDefault;

use dioxus_native_core::{
    node_ref::{AttributeMaskBuilder, NodeMaskBuilder},
    prelude::*,
    real_dom::NodeImmutable,
    utils::{IteratorMovement, PersistantElementIter},
};
use dioxus_native_core_macro::partial_derive_state;
use once_cell::sync::Lazy;
use rustc_hash::FxHashSet;
use shipyard::Component;
use shipyard::{Get, ViewMut};

use std::{cmp::Ordering, num::NonZeroU16};

use dioxus_native_core::node_ref::NodeView;

#[derive(Component)]
pub struct Focused(pub bool);

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
        Some(self.cmp(other))
    }
}

impl Ord for FocusLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (FocusLevel::Unfocusable, FocusLevel::Unfocusable) => std::cmp::Ordering::Equal,
            (FocusLevel::Unfocusable, FocusLevel::Focusable) => std::cmp::Ordering::Less,
            (FocusLevel::Unfocusable, FocusLevel::Ordered(_)) => std::cmp::Ordering::Less,
            (FocusLevel::Focusable, FocusLevel::Unfocusable) => std::cmp::Ordering::Greater,
            (FocusLevel::Focusable, FocusLevel::Focusable) => std::cmp::Ordering::Equal,
            (FocusLevel::Focusable, FocusLevel::Ordered(_)) => std::cmp::Ordering::Greater,
            (FocusLevel::Ordered(_), FocusLevel::Unfocusable) => std::cmp::Ordering::Greater,
            (FocusLevel::Ordered(_), FocusLevel::Focusable) => std::cmp::Ordering::Less,
            (FocusLevel::Ordered(a), FocusLevel::Ordered(b)) => a.cmp(b),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default, Component)]
pub(crate) struct Focus {
    pub level: FocusLevel,
}

#[partial_derive_state]
impl State for Focus {
    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        .with_attrs(AttributeMaskBuilder::Some(FOCUS_ATTRIBUTES))
        .with_listeners();

    type ParentDependencies = ();
    type ChildDependencies = ();
    type NodeDependencies = ();

    fn update<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: &SendAnyMap,
    ) -> bool {
        let new = Focus {
            level: if let Some(a) = node_view
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
            } else if node_view
                .listeners()
                .and_then(|mut listeners| {
                    listeners.any(|l| FOCUS_EVENTS.contains(&l)).then_some(())
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

    fn create<'a>(
        node_view: NodeView<()>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> Self {
        let mut myself = Self::default();
        myself.update(node_view, node, parent, children, context);
        myself
    }
}

static FOCUS_EVENTS: Lazy<FxHashSet<&str>> =
    Lazy::new(|| ["keydown", "keypress", "keyup"].into_iter().collect());
const FOCUS_ATTRIBUTES: &[&str] = &["tabindex"];

pub(crate) struct FocusState {
    pub(crate) focus_iter: PersistantElementIter,
    pub(crate) last_focused_id: Option<NodeId>,
    pub(crate) focus_level: FocusLevel,
    pub(crate) dirty: bool,
}

impl FocusState {
    pub fn create(rdom: &mut RealDom) -> Self {
        let focus_iter = PersistantElementIter::create(rdom);
        Self {
            focus_iter,
            last_focused_id: Default::default(),
            focus_level: Default::default(),
            dirty: Default::default(),
        }
    }

    /// Returns true if the focus has changed.
    pub fn progress(&mut self, rdom: &mut RealDom, forward: bool) -> bool {
        if let Some(last) = self.last_focused_id {
            if rdom.get(last).unwrap().get::<PreventDefault>().map(|p| *p)
                == Some(PreventDefault::KeyDown)
            {
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
            if let IteratorMovement::Looped = new.movement() {
                let mut closest_level = None;

                if forward {
                    // find the closest focusable element after the current level
                    rdom.traverse_depth_first(|n| {
                        let node_level = n.get::<Focus>().unwrap().level;
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
                        let node_level = n.get::<Focus>().unwrap().level;
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

            let current_level = rdom.get(new_id).unwrap().get::<Focus>().unwrap().level;
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
            let mut node = rdom.get_mut(id).unwrap();
            if !node.get::<Focus>().unwrap().level.focusable() {
                panic!()
            }
            node.insert(Focused(true));
            if let Some(old) = self.last_focused_id.replace(id) {
                let mut focused_borrow: ViewMut<Focused> = rdom.raw_world().borrow().unwrap();
                let focused = (&mut focused_borrow).get(old).unwrap();
                focused.0 = false;
            }
            // reset the position to the currently focused element
            while self.focus_iter.next(rdom).id() != id {}
            self.dirty = true;
            return true;
        }

        false
    }

    pub(crate) fn set_focus(&mut self, rdom: &mut RealDom, id: NodeId) {
        if let Some(old) = self.last_focused_id.replace(id) {
            let mut node = rdom.get_mut(old).unwrap();
            node.insert(Focused(false));
        }
        let mut node = rdom.get_mut(id).unwrap();
        node.insert(Focused(true));
        self.focus_level = node.get::<Focus>().unwrap().level;
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
