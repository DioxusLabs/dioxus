use crate::passes::{AnyMapLike, TypeErasedPass};
use std::cmp::Ordering;

/// Join two sorted iterators
pub(crate) fn union_ordered_iter<'a>(
    s_iter: impl Iterator<Item = &'a str>,
    o_iter: impl Iterator<Item = &'a str>,
    new_len_guess: usize,
) -> Vec<String> {
    let mut s_peekable = s_iter.peekable();
    let mut o_peekable = o_iter.peekable();
    let mut v = Vec::with_capacity(new_len_guess);
    while let Some(s_i) = s_peekable.peek() {
        while let Some(o_i) = o_peekable.peek() {
            match o_i.cmp(s_i) {
                Ordering::Greater => {
                    break;
                }
                Ordering::Less => {
                    v.push(o_peekable.next().unwrap().to_string());
                }
                Ordering::Equal => {
                    o_peekable.next();
                    break;
                }
            }
        }
        v.push(s_peekable.next().unwrap().to_string());
    }
    for o_i in o_peekable {
        v.push(o_i.to_string());
    }
    for w in v.windows(2) {
        debug_assert!(w[1] > w[0]);
    }
    v
}

/// Do not implement this trait. It is only meant to be derived and used through [crate::real_dom::RealDom].
pub trait State: Default + Clone + AnyMapLike + 'static {
    #[doc(hidden)]
    fn create_passes() -> Box<[TypeErasedPass<Self>]>;
}
