/// This is a interface for a tree with the ability to jump to a specific node
pub trait Traversable {
    type Id: Copy;
    type Node;

    fn height(&self, id: Self::Id) -> Option<u16>;

    fn get(&self, id: Self::Id) -> Option<&Self::Node>;
    fn get_mut(&mut self, id: Self::Id) -> Option<&mut Self::Node>;

    fn children(&self, node: Self::Id) -> &[Self::Id];
    fn parent(&self, node: Self::Id) -> Option<Self::Id>;

    fn map<N, F: Fn(&Self::Node) -> &N, FMut: Fn(&mut Self::Node) -> &mut N>(
        &mut self,
        f: F,
        f_mut: FMut,
    ) -> Map<Self, N, F, FMut>
    where
        Self: Sized,
    {
        Map {
            tree: self,
            f,
            f_mut,
        }
    }

    // this is safe because no node will have itself as it's parent
    fn get_node_parent_mut(
        &mut self,
        id: Self::Id,
    ) -> (Option<&mut Self::Node>, Option<&mut Self::Node>) {
        let node = self.get_mut(id).map(|n| n as *mut _);
        let parent = self
            .parent(id)
            .and_then(|n| self.get_mut(n))
            .map(|n| n as *mut _);
        unsafe { (node.map(|n| &mut *n), parent.map(|n| &mut *n)) }
    }

    // this is safe because no node will have itself as a child
    fn get_node_children_mut(
        &mut self,
        id: Self::Id,
    ) -> (Option<&mut Self::Node>, Vec<&mut Self::Node>) {
        let node = self.get_mut(id).map(|n| n as *mut _);
        let mut children = Vec::new();
        let children_indexes = self.children(id).to_vec();
        for id in children_indexes {
            if let Some(n) = self.get_mut(id) {
                children.push(unsafe { &mut *(n as *mut _) });
            }
        }
        unsafe { (node.map(|n| &mut *n), children) }
    }
}

/// Maps one type of tree to another. Similar to [std::iter::Map].
pub struct Map<
    'a,
    T: Traversable,
    N,
    F: Fn(&<T as Traversable>::Node) -> &N,
    FMut: Fn(&mut <T as Traversable>::Node) -> &mut N,
> {
    f: F,
    f_mut: FMut,
    tree: &'a mut T,
}

impl<
        'a,
        T: Traversable,
        N,
        F: Fn(&<T as Traversable>::Node) -> &N,
        FMut: Fn(&mut <T as Traversable>::Node) -> &mut N,
    > Traversable for Map<'a, T, N, F, FMut>
{
    type Id = <T as Traversable>::Id;
    type Node = N;

    fn height(&self, id: Self::Id) -> Option<u16> {
        self.tree.height(id)
    }

    fn get(&self, id: Self::Id) -> Option<&Self::Node> {
        self.tree.get(id).map(&self.f)
    }

    fn get_mut(&mut self, id: Self::Id) -> Option<&mut Self::Node> {
        self.tree.get_mut(id).map(&self.f_mut)
    }

    fn children(&self, id: Self::Id) -> &[Self::Id] {
        self.tree.children(id)
    }

    fn parent(&self, id: Self::Id) -> Option<Self::Id> {
        self.tree.parent(id)
    }
}
