use dioxus::prelude::*;
use dioxus_native_core::node::NodeType;
use dioxus_native_core::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::any::TypeId;
use std::sync::{Arc, Mutex};

fn create_blank_element() -> NodeType {
    NodeType::Element(ElementNode {
        tag: "div".to_owned(),
        namespace: None,
        attributes: FxHashMap::default(),
        listeners: FxHashSet::default(),
    })
}

#[test]
fn node_pass() {
    #[derive(Debug, Default, Clone, PartialEq)]
    struct Number(i32);

    impl Pass for Number {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = ();
        const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 += 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([Number::to_type_erased()]));
    tree.update_state(SendAnyMap::new(), false);

    assert_eq!(tree.get(tree.root_id()).unwrap().get(), Some(&Number(1)));

    // mark the node as dirty
    tree.get_mut(tree.root_id()).unwrap().get_mut::<Number>();

    tree.update_state(SendAnyMap::new(), false);
    assert_eq!(tree.get(tree.root_id()).unwrap().get(), Some(&Number(2)));
}

#[test]
fn dependant_node_pass() {
    #[derive(Debug, Default, Clone, PartialEq)]
    struct AddNumber(i32);

    impl Pass for AddNumber {
        type ChildDependencies = ();
        type NodeDependencies = (SubtractNumber,);
        type ParentDependencies = ();
        const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 += 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    #[derive(Debug, Default, Clone, PartialEq)]
    struct SubtractNumber(i32);

    impl Pass for SubtractNumber {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = ();
        const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 -= 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([
        AddNumber::to_type_erased(),
        SubtractNumber::to_type_erased(),
    ]));
    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(1)));
    assert_eq!(root.get(), Some(&SubtractNumber(-1)));

    // mark the subtract state as dirty, it should update the add state
    tree.get_mut(tree.root_id())
        .unwrap()
        .get_mut::<SubtractNumber>();

    tree.update_state(SendAnyMap::new(), false);
    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(2)));
    assert_eq!(root.get(), Some(&SubtractNumber(-2)));

    // mark the add state as dirty, it should ~not~ update the subtract state
    tree.get_mut(tree.root_id()).unwrap().get_mut::<AddNumber>();

    tree.update_state(SendAnyMap::new(), false);
    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(3)));
    assert_eq!(root.get(), Some(&SubtractNumber(-2)));
}

#[test]
fn independant_node_pass() {
    #[derive(Debug, Default, Clone, PartialEq)]
    struct AddNumber(i32);

    impl Pass for AddNumber {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = ();

        const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 += 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    #[derive(Debug, Default, Clone, PartialEq)]
    struct SubtractNumber(i32);

    impl Pass for SubtractNumber {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = ();

        const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 -= 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([
        AddNumber::to_type_erased(),
        SubtractNumber::to_type_erased(),
    ]));
    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(1)));
    assert_eq!(root.get(), Some(&SubtractNumber(-1)));

    // mark the subtract state as dirty, it should ~not~ update the add state
    tree.get_mut(tree.root_id())
        .unwrap()
        .get_mut::<SubtractNumber>();

    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(1)));
    assert_eq!(root.get(), Some(&SubtractNumber(-2)));

    // mark the add state as dirty, it should ~not~ update the subtract state
    tree.get_mut(tree.root_id()).unwrap().get_mut::<AddNumber>();

    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(2)));
    assert_eq!(root.get(), Some(&SubtractNumber(-2)));
}

#[test]
fn down_pass() {
    #[derive(Debug, Clone, PartialEq)]
    struct AddNumber(i32);

    impl Default for AddNumber {
        fn default() -> Self {
            Self(1)
        }
    }

    impl Pass for AddNumber {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = (AddNumber,);

        const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            dbg!(parent);
            dbg!(node_view);
            if let Some((parent,)) = parent {
                self.0 += parent.0;
            }
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([AddNumber::to_type_erased()]));
    let mut grandchild1 = tree.create_node(create_blank_element(), true);
    let grandchild1 = grandchild1.id();
    let mut child1 = tree.create_node(create_blank_element(), true);
    child1.add_child(grandchild1);
    let child1 = child1.id();
    let mut grandchild2 = tree.create_node(create_blank_element(), true);
    let grandchild2 = grandchild2.id();
    let mut child2 = tree.create_node(create_blank_element(), true);
    child2.add_child(grandchild2);
    let child2 = child2.id();
    let mut parent = tree.get_mut(tree.root_id()).unwrap();
    parent.add_child(child1);
    parent.add_child(child2);

    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    dbg!(root.id());
    assert_eq!(root.get(), Some(&AddNumber(1)));

    let child1 = tree.get(child1).unwrap();
    dbg!(child1.id());
    dbg!(child1.parent().unwrap().get::<AddNumber>());
    assert_eq!(child1.get(), Some(&AddNumber(2)));

    let grandchild1 = tree.get(grandchild1).unwrap();
    assert_eq!(grandchild1.get(), Some(&AddNumber(3)));

    let child2 = tree.get(child2).unwrap();
    assert_eq!(child2.get(), Some(&AddNumber(2)));

    let grandchild2 = tree.get(grandchild2).unwrap();
    assert_eq!(grandchild2.get(), Some(&AddNumber(3)));
}

#[test]
fn up_pass() {
    // Tree before:
    // 1=\
    //   1=\
    //     1
    //   1=\
    //     1
    // Tree after:
    // 4=\
    //   2=\
    //     1
    //   2=\
    //     1

    #[derive(Debug, Clone, PartialEq)]
    struct AddNumber(i32);

    impl Pass for AddNumber {
        type ChildDependencies = (AddNumber,);
        type NodeDependencies = ();
        type ParentDependencies = ();

        const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            if let Some(children) = children {
                self.0 += children.iter().map(|(i,)| i.0).sum::<i32>();
            }
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self(1);
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([AddNumber::to_type_erased()]));
    let mut grandchild1 = tree.create_node(create_blank_element(), true);
    let grandchild1 = grandchild1.id();
    let mut child1 = tree.create_node(create_blank_element(), true);
    child1.add_child(grandchild1);
    let child1 = child1.id();
    let mut grandchild2 = tree.create_node(create_blank_element(), true);
    let grandchild2 = grandchild2.id();
    let mut child2 = tree.create_node(create_blank_element(), true);
    child2.add_child(grandchild2);
    let child2 = child2.id();
    let mut parent = tree.get_mut(tree.root_id()).unwrap();
    parent.add_child(child1);
    parent.add_child(child2);

    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(5)));

    let child1 = tree.get(child1).unwrap();
    assert_eq!(child1.get(), Some(&AddNumber(2)));

    let grandchild1 = tree.get(grandchild1).unwrap();
    assert_eq!(grandchild1.get(), Some(&AddNumber(1)));

    let child2 = tree.get(child2).unwrap();
    assert_eq!(child2.get(), Some(&AddNumber(2)));

    let grandchild2 = tree.get(grandchild2).unwrap();
    assert_eq!(grandchild2.get(), Some(&AddNumber(1)));
}
