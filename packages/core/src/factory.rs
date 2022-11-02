use std::{cell::Cell, fmt::Arguments};

use crate::{
    arena::ElementId,
    innerlude::{DynamicNode, DynamicNodeKind},
    LazyNodes, ScopeState, VNode,
};

impl ScopeState {
    /// Create some text that's allocated along with the other vnodes
    ///
    pub fn text<'a>(&'a self, args: Arguments) -> DynamicNode<'a> {
        let (text, _) = self.raw_text(args);

        DynamicNode {
            kind: DynamicNodeKind::Text {
                id: Cell::new(ElementId(0)),
                value: text,
            },
            path: &[0],
        }
    }

    pub fn raw_text<'a>(&'a self, args: Arguments) -> (&'a str, bool) {
        match args.as_str() {
            Some(static_str) => (static_str, true),
            None => {
                use bumpalo::core_alloc::fmt::Write;
                let mut str_buf = bumpalo::collections::String::new_in(self.bump());
                str_buf.write_fmt(args).unwrap();
                (str_buf.into_bump_str(), false)
            }
        }
    }

    pub fn fragment_from_iter<'a, I, F: IntoVnode<'a, I>>(
        &'a self,
        it: impl IntoIterator<Item = F>,
    ) -> DynamicNode {
        let mut bump_vec = bumpalo::vec![in self.bump();];

        for item in it {
            bump_vec.push(item.into_dynamic_node(self));
        }

        DynamicNode {
            path: &[0, 0],
            kind: crate::innerlude::DynamicNodeKind::Fragment {
                children: bump_vec.into_bump_slice(),
            },
        }
    }
}

pub trait IntoVnode<'a, A = ()> {
    fn into_dynamic_node(self, cx: &'a ScopeState) -> VNode<'a>;
}

impl<'a, 'b> IntoVnode<'a> for LazyNodes<'a, 'b> {
    fn into_dynamic_node(self, cx: &'a ScopeState) -> VNode<'a> {
        self.call(cx)
    }
}
