use anymap::AnyMap;
use dioxus::prelude::*;
use dioxus_native_core::node_ref::*;
use dioxus_native_core::real_dom::*;
use dioxus_native_core::state::{ChildDepState, NodeDepState, ParentDepState, State};
use dioxus_native_core_macro::State;

macro_rules! dep {
    ( child( $name:ty, $dep:ty ) ) => {
        impl ChildDepState for $name {
            type Ctx = ();
            type DepState = $dep;
            const NODE_MASK: NodeMask = NodeMask::ALL;
            fn reduce<'a>(
                &mut self,
                _: NodeView,
                _: impl Iterator<Item = &'a Self::DepState>,
                _: &Self::Ctx,
            ) -> bool
            where
                Self::DepState: 'a,
            {
                self.0 += 1;
                true
            }
        }
    };

    ( parent( $name:ty, $dep:ty ) ) => {
        impl ParentDepState for $name {
            type Ctx = ();
            type DepState = $dep;
            const NODE_MASK: NodeMask = NodeMask::ALL;
            fn reduce(
                &mut self,
                _: NodeView,
                _: Option<&Self::DepState>,
                _: &Self::Ctx,
            ) -> bool {
                self.0 += 1;
                true
            }
        }
    };

    ( node( $name:ty, ($($l:lifetime),*), $dep:ty ) ) => {
        impl<$($l),*> NodeDepState<$dep> for $name {
            type Ctx = ();
            const NODE_MASK: NodeMask = NodeMask::ALL;
            fn reduce(
                &mut self,
                _: NodeView,
                _: $dep,
                _: &Self::Ctx,
            ) -> bool {
                self.0 += 1;
                true
            }
        }
    };
}

macro_rules! test_state{
    ( $s:ty, child: ( $( $child:ident ),* ), node: ( $( $node:ident ),* ), parent: ( $( $parent:ident ),* ) ) => {
        #[test]
        fn state_reduce_initally_called_minimally() {
            #[allow(non_snake_case)]
            fn Base(cx: Scope) -> Element {
                render!(div {
                    div{
                        div{
                            p{}
                        }
                        p{
                            "hello"
                        }
                        div{
                            h1{}
                        }
                        p{
                            "world"
                        }
                    }
                })
            }

            let vdom = VirtualDom::new(Base);

            let mutations = vdom.create_vnodes(rsx! {
                div {
                    div{
                        div{
                            p{}
                        }
                        p{
                            "hello"
                        }
                        div{
                            h1{}
                        }
                        p{
                            "world"
                        }
                    }
                }
            });

            let mut dom: RealDom<$s> = RealDom::new();

            let nodes_updated = dom.apply_mutations(vec![mutations]);
            let _to_rerender = dom.update_state(nodes_updated, AnyMap::new());

            dom.traverse_depth_first(|n| {
                $(
                    assert_eq!(n.state.$child.0, 1);
                )*
                $(
                    assert_eq!(n.state.$node.0, 1);
                )*
                $(
                    assert_eq!(n.state.$parent.0, 1);
                )*
            });
        }
    }
}

mod node_depends_on_child_and_parent {
    use super::*;
    #[derive(Debug, Clone, Default, PartialEq)]
    struct Node(i32);
    dep!(node(Node,  ('a, 'b), (&'a Child, &'b Parent)));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Child(i32);
    dep!(child(Child, Child));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Parent(i32);
    dep!(parent(Parent, Parent));

    #[derive(Debug, Clone, Default, State)]
    struct StateTester {
        #[node_dep_state((child, parent))]
        node: Node,
        #[child_dep_state(child)]
        child: Child,
        #[parent_dep_state(parent)]
        parent: Parent,
    }

    test_state!(StateTester, child: (child), node: (node), parent: (parent));
}

mod child_depends_on_node_that_depends_on_parent {
    use super::*;
    #[derive(Debug, Clone, Default, PartialEq)]
    struct Node(i32);
    dep!(node(Node, ('a), (&'a Parent,)));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Child(i32);
    dep!(child(Child, Node));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Parent(i32);
    dep!(parent(Parent, Parent));

    #[derive(Debug, Clone, Default, State)]
    struct StateTester {
        #[node_dep_state(parent)]
        node: Node,
        #[child_dep_state(node)]
        child: Child,
        #[parent_dep_state(parent)]
        parent: Parent,
    }

    test_state!(StateTester, child: (child), node: (node), parent: (parent));
}

mod parent_depends_on_node_that_depends_on_child {
    use super::*;
    #[derive(Debug, Clone, Default, PartialEq)]
    struct Node(i32);
    dep!(node(Node, ('a), (&'a Child,)));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Child(i32);
    dep!(child(Child, Child));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Parent(i32);
    dep!(parent(Parent, Node));

    #[derive(Debug, Clone, Default, State)]
    struct StateTester {
        #[node_dep_state(child)]
        node: Node,
        #[child_dep_state(child)]
        child: Child,
        #[parent_dep_state(node)]
        parent: Parent,
    }

    test_state!(StateTester, child: (child), node: (node), parent: (parent));
}

mod node_depends_on_other_node_state {
    use super::*;
    #[derive(Debug, Clone, Default, PartialEq)]
    struct Node1(i32);
    dep!(node(Node1, ('a), (&'a Node2,)));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Node2(i32);
    dep!(node(Node2, (), ()));

    #[derive(Debug, Clone, Default, State)]
    struct StateTester {
        #[node_dep_state((node2))]
        node1: Node1,
        #[node_dep_state()]
        node2: Node2,
    }

    test_state!(StateTester, child: (), node: (node1, node2), parent: ());
}

mod node_child_and_parent_state_depends_on_self {
    use super::*;
    #[derive(Debug, Clone, Default, PartialEq)]
    struct Node(i32);
    dep!(node(Node, (), ()));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Child(i32);
    dep!(child(Child, Child));

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Parent(i32);
    dep!(parent(Parent, Parent));

    #[derive(Debug, Clone, Default, State)]
    struct StateTester {
        #[node_dep_state()]
        node: Node,
        #[child_dep_state(child)]
        child: Child,
        #[parent_dep_state(parent)]
        parent: Parent,
    }

    test_state!(StateTester, child: (child), node: (node), parent: (parent));
}
