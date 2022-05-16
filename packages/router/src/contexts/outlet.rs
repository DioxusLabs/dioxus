use std::collections::BTreeMap;

/// A context used by outlets to determine how deeply nested they are.
#[derive(Clone, Default)]
pub(crate) struct OutletContext {
    /// The depth of the outlet providing the context.
    pub(crate) depth: Option<usize>,
    /// Same as `depth` but for named outlets.
    pub(crate) named_depth: BTreeMap<String, usize>,
}

impl OutletContext {
    /// Create a new [`OutletContext`] and set the depth for `name` to 0.
    pub(crate) fn new(name: Option<&str>) -> Self {
        let mut new = Self::default();
        new.set_depth(name, 0);
        new
    }

    /// Get the depth for an [`Outlet`] consuming the [`OutletContext`].
    ///
    /// [`Outlet`]: crate::components::Outlet
    pub(crate) fn get_depth(&self, name: Option<&str>) -> usize {
        match name {
            None => self.depth.map_or(0, |d| d + 1),
            Some(name) => self.named_depth.get(name).map_or(0, |d| d + 1),
        }
    }

    /// Set the depth of the [`Outlet`] providing the [`OutletContext`].
    ///
    /// [`Outlet`]: crate::components::Outlet
    pub(crate) fn set_depth(&mut self, name: Option<&str>, depth: usize) {
        match name {
            None => self.depth = Some(depth),
            Some(name) => {
                self.named_depth.insert(name.to_string(), depth);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_named() {
        let new = OutletContext::new(Some("test"));

        assert_eq!(None, new.depth);
        assert_eq!(1, new.named_depth.len());
        assert_eq!(0, new.named_depth["test"]);
    }

    #[test]
    fn new_nameless() {
        let new = OutletContext::new(None);

        assert_eq!(Some(0), new.depth);
        assert!(new.named_depth.is_empty());
    }

    #[test]
    fn get_depth() {
        let ctx = test_context();

        assert_eq!(1, ctx.get_depth(None));
        assert_eq!(4, ctx.get_depth(Some("test")));
        assert_eq!(0, ctx.get_depth(Some("new")));
    }

    #[test]
    fn set_depth() {
        let mut ctx = test_context();

        ctx.set_depth(None, ctx.get_depth(None));
        ctx.set_depth(Some("test"), ctx.get_depth(Some("test")));
        ctx.set_depth(Some("new"), ctx.get_depth(Some("new")));

        assert_eq!(Some(1), ctx.depth);
        assert_eq!(2, ctx.named_depth.len());
        assert_eq!(4, ctx.named_depth["test"]);
        assert_eq!(0, ctx.named_depth["new"]);
    }

    fn test_context() -> OutletContext {
        OutletContext {
            depth: Some(0),
            named_depth: {
                let mut d = BTreeMap::new();
                d.insert(String::from("test"), 3);
                d
            },
        }
    }
}
