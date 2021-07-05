fn main() {}

struct Title(String);

struct Position([f32; 3]);

struct Velocity([f32; 3]);

type Batch<T> = fn(&mut T) -> ();

static Atom: Batch<(Title, Position, Velocity)> = |_| {};

enum VNode<'a> {
    El(El<'a>),
    Text(&'a str),
    Fragment(&'a [VNode<'a>]),
}
struct El<'a> {
    name: &'static str,
    key: Option<&'a str>,
    attrs: &'a [(&'static str, AttrType<'a>)],
    children: &'a [El<'a>],
}
enum AttrType<'a> {
    Numeric(usize),
    Text(&'a str),
}

fn example() {
    use AttrType::Numeric;
    let el = El {
        name: "div",
        attrs: &[("type", Numeric(10)), ("type", Numeric(10))],
        key: None,
        children: &[],
    };
}

use dioxus::prelude::bumpalo::Bump;
trait IntoVnode {
    fn into_vnode<'a>(self, b: &'a Bump) -> VNode<'a>;
}

impl<'a> IntoIterator for VNode<'a> {
    type Item = VNode<'a>;
    type IntoIter = std::iter::Once<VNode<'a>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

fn take_iterable<F: IntoVnode>(f: impl IntoIterator<Item = F>) {
    let iter = f.into_iter();
    let b = Bump::new();
    for f in iter {
        let v = f.into_vnode(&b);
    }
}
