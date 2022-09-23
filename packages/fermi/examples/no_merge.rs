use std::{any::type_name, marker::PhantomData};

use fermi::{Atom, AtomBuilder};
use im_rc::HashMap;

static TITLE: Batom<String> = atom(|_| "The Biggest Name in Hollywood".into());

// static AGE0: Atom<i32> = |_| 10;

// static AGE1: Atom<HashMap<String, String>> = |_| HashMap::default();

// static AGE2: Atom<HashMap<String, String>> = |_| HashMap::default();

// static AGE3: Atom<i32> = atom(|_| 10);

#[derive(Debug, Clone, Copy)]
struct Batom<I> {
    f: fn(AtomBuilder) -> I,
    g: &'static str,
}

enum BuilderSyntax<V> {
    With(fn(AtomBuilder) -> V),
    Without(fn() -> V),
}

const fn atom<I>(f: fn(AtomBuilder) -> I) -> Batom<I> {
    let g = concat!(file!(), ":", line!());
    Batom { f, g }
}

fn main() {
    // dbg!(TITL);
    // dbg!(AGE0);
    // dbg!(AGE1);
    // dbg!(AGE2);
    // dbg!(AGE3);
}
