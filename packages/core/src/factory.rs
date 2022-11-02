use std::fmt::Arguments;

use crate::{innerlude::DynamicNode, LazyNodes, ScopeState, VNode};

impl ScopeState {
    /// Create some text that's allocated along with the other vnodes
    ///
    pub fn text(&self, args: Arguments) -> DynamicNode {
        // let (text, _is_static) = self.raw_text(args);

        // VNode::Text(self.bump.alloc(VText {
        //     text,
        //     id: Default::default(),
        // }))

        todo!()
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
