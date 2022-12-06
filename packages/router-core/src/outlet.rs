use std::collections::BTreeMap;

use crate::Name;

/// Information outlets can use to find out what to render.
///
/// Outlets (which must be implemented by crates tying dioxus-router-core to UI crates) can use this
/// information to find out how deeply nested they are within other outlets, and communicate the
/// same to outlets nested inside them.
#[derive(Debug, Default, Clone)]
pub struct OutletData {
    main: Option<usize>,
    named: BTreeMap<Name, usize>,
}

impl OutletData {
    /// Create some [`OutletData`] nested one level deeper and get the current depth.
    ///
    /// ```rust
    /// # use dioxus_router_core::{Name, OutletData};
    /// let mut d = OutletData::default();
    /// let (m, a, n);
    /// (m, d) = d.next(&None);
    /// (a, d) = d.next(&None);
    /// (n, d) = d.next(&Some(Name::of::<bool>()));
    ///
    /// assert_eq!(m, 0);
    /// assert_eq!(a, 1);
    /// assert_eq!(n, 0);
    /// ```
    pub fn next(&self, name: &Option<Name>) -> (usize, Self) {
        let mut next = self.clone();

        let depth = next.depth(name).map(|d| d + 1).unwrap_or(0);

        next.set_depth(name, depth);

        (depth, next)
    }

    /// Get the current depth for `name`.
    ///
    /// ```rust
    /// # use dioxus_router_core::OutletData;
    /// let mut d = OutletData::default();
    /// let b = d.depth(&None);
    /// d.set_depth(&None, 18);
    /// let a = d.depth(&None);
    ///
    /// assert_eq!(b, None);
    /// assert_eq!(a, Some(18));
    /// ```
    pub fn depth(&self, name: &Option<Name>) -> Option<usize> {
        match name {
            None => self.main,
            Some(n) => self.named.get(n).copied(),
        }
    }

    /// Set the depth for `name`.
    ///
    /// ```rust
    /// # use dioxus_router_core::OutletData;
    /// let mut d = OutletData::default();
    /// let b = d.depth(&None);
    /// d.set_depth(&None, 18);
    /// let a = d.depth(&None);
    ///
    /// assert_eq!(b, None);
    /// assert_eq!(a, Some(18));
    /// ```
    pub fn set_depth(&mut self, name: &Option<Name>, depth: usize) {
        match name {
            None => self.main = Some(depth),
            Some(n) => _ = self.named.insert(n.clone(), depth),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_data() -> OutletData {
        let mut named = BTreeMap::new();
        named.insert(Name::of::<bool>(), 0);
        named.insert(Name::of::<u8>(), 8);
        named.insert(Name::of::<u16>(), 16);
        named.insert(Name::of::<u32>(), 32);
        named.insert(Name::of::<u64>(), 64);

        OutletData {
            main: Some(18),
            named,
        }
    }

    #[test]
    fn default() {
        let d = OutletData::default();

        assert!(d.main.is_none());
        assert!(d.named.is_empty());
    }

    #[test]
    fn depth() {
        let td = test_data();

        assert_eq!(td.depth(&None), Some(18));
        assert_eq!(td.depth(&Some(Name::of::<bool>())), Some(0));

        assert_eq!(td.depth(&Some(Name::of::<u8>())), Some(8));
        assert_eq!(td.depth(&Some(Name::of::<u16>())), Some(16));
        assert_eq!(td.depth(&Some(Name::of::<u32>())), Some(32));
        assert_eq!(td.depth(&Some(Name::of::<u64>())), Some(64));

        assert_eq!(td.depth(&Some(Name::of::<i8>())), None);
        assert_eq!(td.depth(&Some(Name::of::<i16>())), None);
        assert_eq!(td.depth(&Some(Name::of::<i32>())), None);
        assert_eq!(td.depth(&Some(Name::of::<i64>())), None);
    }

    #[test]
    fn set_depth() {
        let mut td = test_data();

        // set
        td.set_depth(&None, 0);
        td.set_depth(&Some(Name::of::<bool>()), 1);

        td.set_depth(&Some(Name::of::<u8>()), 2);
        td.set_depth(&Some(Name::of::<u16>()), 4);
        td.set_depth(&Some(Name::of::<u32>()), 8);
        td.set_depth(&Some(Name::of::<u64>()), 16);

        td.set_depth(&Some(Name::of::<i8>()), 32);
        td.set_depth(&Some(Name::of::<i16>()), 64);
        td.set_depth(&Some(Name::of::<i32>()), 128);
        td.set_depth(&Some(Name::of::<i64>()), 256);

        // check
        assert_eq!(td.depth(&None), Some(0));
        assert_eq!(*td.named.get(&Name::of::<bool>()).unwrap(), 1);

        assert_eq!(*td.named.get(&Name::of::<u8>()).unwrap(), 2);
        assert_eq!(*td.named.get(&Name::of::<u16>()).unwrap(), 4);
        assert_eq!(*td.named.get(&Name::of::<u32>()).unwrap(), 8);
        assert_eq!(*td.named.get(&Name::of::<u64>()).unwrap(), 16);

        assert_eq!(*td.named.get(&Name::of::<i8>()).unwrap(), 32);
        assert_eq!(*td.named.get(&Name::of::<i16>()).unwrap(), 64);
        assert_eq!(*td.named.get(&Name::of::<i32>()).unwrap(), 128);
        assert_eq!(*td.named.get(&Name::of::<i64>()).unwrap(), 256);
    }

    #[test]
    fn next() {
        let td = test_data();

        let (current, next) = td.next(&None);
        assert_eq!(current, 19);
        assert_eq!(next.depth(&None), Some(19));

        let (current, next) = td.next(&Some(Name::of::<bool>()));
        assert_eq!(current, 1);
        assert_eq!(*next.named.get(&Name::of::<bool>()).unwrap(), 1);

        let (current, next) = td.next(&Some(Name::of::<u8>()));
        assert_eq!(current, 9);
        assert_eq!(*next.named.get(&Name::of::<u8>()).unwrap(), 9);

        let (current, next) = td.next(&Some(Name::of::<u16>()));
        assert_eq!(current, 17);
        assert_eq!(*next.named.get(&Name::of::<u16>()).unwrap(), 17);

        let (current, next) = td.next(&Some(Name::of::<u32>()));
        assert_eq!(current, 33);
        assert_eq!(*next.named.get(&Name::of::<u32>()).unwrap(), 33);

        let (current, next) = td.next(&Some(Name::of::<u64>()));
        assert_eq!(current, 65);
        assert_eq!(*next.named.get(&Name::of::<u64>()).unwrap(), 65);

        let (current, next) = td.next(&Some(Name::of::<i8>()));
        assert_eq!(current, 0);
        assert_eq!(*next.named.get(&Name::of::<i8>()).unwrap(), 0);

        let (current, next) = td.next(&Some(Name::of::<i16>()));
        assert_eq!(current, 0);
        assert_eq!(*next.named.get(&Name::of::<i16>()).unwrap(), 0);

        let (current, next) = td.next(&Some(Name::of::<i32>()));
        assert_eq!(current, 0);
        assert_eq!(*next.named.get(&Name::of::<i32>()).unwrap(), 0);

        let (current, next) = td.next(&Some(Name::of::<i64>()));
        assert_eq!(current, 0);
        assert_eq!(*next.named.get(&Name::of::<i64>()).unwrap(), 0);
    }
}
