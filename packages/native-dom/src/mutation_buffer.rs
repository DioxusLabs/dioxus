//! Buffering of Dioxus mutations so that they can be applied after the VirtualDom has run
use dioxus_core::{AnyValue, BorrowedAttributeValue, ElementId, WriteMutations};
use std::rc::Rc;

/// A span of [`MutationBuffer::strings`]
#[derive(Clone, Copy, Debug)]
struct StrSpan {
    start: u32,
    end: u32,
}

enum BufferedAttributeValue {
    Text(StrSpan),
    Float(f64),
    Int(i64),
    Bool(bool),
    Any(Rc<dyn AnyValue>),
    None,
}

enum BufferedMutation {
    PushId(ElementId),
    SetId(ElementId),
    Child(usize),
    Pop,
    CreateElement {
        tag: StrSpan,
        ns: Option<StrSpan>,
    },
    CreateText(StrSpan),
    Clone,
    AppendChildren(usize),
    ReplaceWith(usize),
    InsertAfter(usize),
    InsertBefore(usize),
    SetAttribute {
        name: StrSpan,
        ns: Option<StrSpan>,
        value: BufferedAttributeValue,
    },
    SetText(StrSpan),
    AddEventListener(StrSpan),
    RemoveEventListener(StrSpan),
    Remove,
}

/// Records the mutations written by the VirtualDom so that they can be applied to the
/// document once the VirtualDom (and any user code that it runs) has finished running.
///
/// The buffer keeps its allocations when it is replayed so that they are reused by later renders.
#[derive(Default)]
pub(crate) struct MutationBuffer {
    mutations: Vec<BufferedMutation>,
    /// The concatenated string arguments of `mutations`
    strings: String,
}

impl std::fmt::Debug for MutationBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutationBuffer")
            .field("len", &self.mutations.len())
            .finish_non_exhaustive()
    }
}

impl MutationBuffer {
    fn push_str(&mut self, value: &str) -> StrSpan {
        let start = self.strings.len();
        self.strings.push_str(value);
        let end = u32::try_from(self.strings.len()).expect("mutation buffer exceeds 4GB");
        StrSpan {
            start: start as u32,
            end,
        }
    }

    /// Apply the buffered mutations to `to` and empty the buffer
    pub(crate) fn replay(&mut self, to: &mut impl WriteMutations) {
        let strings = self.strings.as_str();
        let str = |span: StrSpan| &strings[span.start as usize..span.end as usize];

        for mutation in self.mutations.drain(..) {
            match mutation {
                BufferedMutation::PushId(id) => to.push_id(id),
                BufferedMutation::SetId(id) => to.set_id(id),
                BufferedMutation::Child(index) => to.child(index),
                BufferedMutation::Pop => to.pop(),
                BufferedMutation::CreateElement { tag, ns } => {
                    to.create_element(str(tag), ns.map(str))
                }
                BufferedMutation::CreateText(value) => to.create_text(str(value)),
                BufferedMutation::Clone => to.clone(),
                BufferedMutation::AppendChildren(m) => to.append_children(m),
                BufferedMutation::ReplaceWith(m) => to.replace_with(m),
                BufferedMutation::InsertAfter(m) => to.insert_after(m),
                BufferedMutation::InsertBefore(m) => to.insert_before(m),
                BufferedMutation::SetAttribute { name, ns, value } => {
                    let value = match &value {
                        BufferedAttributeValue::Text(value) => {
                            BorrowedAttributeValue::Text(str(*value))
                        }
                        BufferedAttributeValue::Float(value) => {
                            BorrowedAttributeValue::Float(*value)
                        }
                        BufferedAttributeValue::Int(value) => BorrowedAttributeValue::Int(*value),
                        BufferedAttributeValue::Bool(value) => BorrowedAttributeValue::Bool(*value),
                        BufferedAttributeValue::Any(value) => BorrowedAttributeValue::Any(value),
                        BufferedAttributeValue::None => BorrowedAttributeValue::None,
                    };
                    to.set_attribute(str(name), ns.map(str), value);
                }
                BufferedMutation::SetText(value) => to.set_text(str(value)),
                BufferedMutation::AddEventListener(name) => to.add_event_listener(str(name)),
                BufferedMutation::RemoveEventListener(name) => to.remove_event_listener(str(name)),
                BufferedMutation::Remove => to.remove(),
            }
        }

        self.strings.clear();
    }
}

impl WriteMutations for MutationBuffer {
    fn push_id(&mut self, id: ElementId) {
        self.mutations.push(BufferedMutation::PushId(id));
    }

    fn set_id(&mut self, id: ElementId) {
        self.mutations.push(BufferedMutation::SetId(id));
    }

    fn child(&mut self, index: usize) {
        self.mutations.push(BufferedMutation::Child(index));
    }

    fn pop(&mut self) {
        self.mutations.push(BufferedMutation::Pop);
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        let tag = self.push_str(tag);
        let ns = ns.map(|ns| self.push_str(ns));
        self.mutations
            .push(BufferedMutation::CreateElement { tag, ns });
    }

    fn create_text(&mut self, value: &str) {
        let value = self.push_str(value);
        self.mutations.push(BufferedMutation::CreateText(value));
    }

    fn clone(&mut self) {
        self.mutations.push(BufferedMutation::Clone);
    }

    fn append_children(&mut self, m: usize) {
        self.mutations.push(BufferedMutation::AppendChildren(m));
    }

    fn replace_with(&mut self, m: usize) {
        self.mutations.push(BufferedMutation::ReplaceWith(m));
    }

    fn insert_after(&mut self, m: usize) {
        self.mutations.push(BufferedMutation::InsertAfter(m));
    }

    fn insert_before(&mut self, m: usize) {
        self.mutations.push(BufferedMutation::InsertBefore(m));
    }

    fn set_attribute(&mut self, name: &str, ns: Option<&str>, value: BorrowedAttributeValue<'_>) {
        let name = self.push_str(name);
        let ns = ns.map(|ns| self.push_str(ns));
        let value = match value {
            BorrowedAttributeValue::Text(value) => {
                BufferedAttributeValue::Text(self.push_str(value))
            }
            BorrowedAttributeValue::Float(value) => BufferedAttributeValue::Float(value),
            BorrowedAttributeValue::Int(value) => BufferedAttributeValue::Int(value),
            BorrowedAttributeValue::Bool(value) => BufferedAttributeValue::Bool(value),
            BorrowedAttributeValue::Any(value) => BufferedAttributeValue::Any(Rc::clone(value)),
            BorrowedAttributeValue::None => BufferedAttributeValue::None,
        };
        self.mutations
            .push(BufferedMutation::SetAttribute { name, ns, value });
    }

    fn set_text(&mut self, value: &str) {
        let value = self.push_str(value);
        self.mutations.push(BufferedMutation::SetText(value));
    }

    fn add_event_listener(&mut self, name: &str) {
        let name = self.push_str(name);
        self.mutations
            .push(BufferedMutation::AddEventListener(name));
    }

    fn remove_event_listener(&mut self, name: &str) {
        let name = self.push_str(name);
        self.mutations
            .push(BufferedMutation::RemoveEventListener(name));
    }

    fn remove(&mut self) {
        self.mutations.push(BufferedMutation::Remove);
    }
}
