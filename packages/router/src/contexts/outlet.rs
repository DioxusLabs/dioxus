use std::collections::BTreeMap;

/// A context used by outlets to determine how deeply nested they are.
#[derive(Clone, Debug, Default)]
pub(crate) struct OutletContext {
    /// The depth of the outlet providing the context.
    pub(crate) depth: Option<usize>,
    /// Same as `depth` but for named outlets.
    pub(crate) named_depth: BTreeMap<String, usize>,
}

impl OutletContext {
    /// Get the depth for an [`Outlet`] consuming the [`OutletContext`].
    ///
    /// [`Outlet`]: crate::components::Outlet
    pub(crate) fn get_depth(&self, name: Option<&str>) -> usize {
        match name {
            None => self.depth.map(|d| d + 1),
            Some(name) => self.named_depth.get(name).map(|d| d + 1),
        }
        .unwrap_or_default()
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
