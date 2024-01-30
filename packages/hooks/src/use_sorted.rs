use std::cmp::Ordering;
use std::ops::DerefMut;

use crate::use_memo;
use dioxus_signals::{ReadOnlySignal, Signal};

pub fn use_sorted<V: 'static, T: PartialEq>(
    collection: impl FnMut() -> Signal<V>,
) -> ReadOnlySignal<Vec<T>>
// pub fn use_sorted<S, I, T>(iterable: impl FnMut() -> Signal<V>) -> ReadOnlySignal<T>
// where
//     S: Into<MaybeSignal<I>>,
//     T: Ord,
//     I: DerefMut<Target = [T]> + Clone + PartialEq,
{
    use_memo(move || {
        todo!()
        // let mut iterable = collection();
        // iterable.sort();
        // iterable
    })
    // let iterable = iterable.into();

    // // use_memo(f)

    // create_memo(move |_| {
    //     let mut iterable = iterable.get();
    //     iterable.sort();
    //     iterable
    // })
    // .into()
}

// /// Version of [`use_sorted`] with a compare function.
// pub fn use_sorted_by<S, I, T, F>(iterable: S, cmp_fn: F) -> Signal<I>
// where
//     S: Into<MaybeSignal<I>>,
//     I: DerefMut<Target = [T]> + Clone + PartialEq,
//     F: FnMut(&T, &T) -> Ordering + Clone + 'static,
// {
//     let iterable = iterable.into();

//     create_memo(move |_| {
//         let mut iterable = iterable.get();
//         iterable.sort_by(cmp_fn.clone());
//         iterable
//     })
//     .into()
// }

// /// Version of [`use_sorted`] by key.
// pub fn use_sorted_by_key<S, I, T, K, F>(iterable: S, key_fn: F) -> Signal<I>
// where
//     S: Into<MaybeSignal<I>>,
//     I: DerefMut<Target = [T]> + Clone + PartialEq,
//     K: Ord,
//     F: FnMut(&T) -> K + Clone + 'static,
// {
//     let iterable = iterable.into();

//     create_memo(move |_| {
//         let mut iterable = iterable.get();
//         iterable.sort_by_key(key_fn.clone());
//         iterable
//     })
//     .into()
// }
