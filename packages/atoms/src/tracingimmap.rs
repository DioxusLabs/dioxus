//! Tracing immap
//! Traces modifications since last generation
//! To reconstruct the history, you will need *all* the generations between the start and end points

use im_rc::HashMap as ImMap;

pub struct TracedHashMap<K, V> {
    inner: ImMap<K, V>,
    generation: u32,
    mods_since_last_gen: Vec<K>,
}

impl<K: Clone, V: Clone> TracedHashMap<K, V> {
    fn next_generation(&self) -> Self {
        Self {
            generation: self.generation + 1,
            inner: self.inner.clone(),
            mods_since_last_gen: vec![],
        }
    }
}

#[test]
fn compare_dos() {
    let map1 = im_rc::hashmap! {3 => 2, 2 => 3};
    let map2 = im_rc::hashmap! {2 => 3, 3 => 2};

    assert_eq!(map1, map2);
}
