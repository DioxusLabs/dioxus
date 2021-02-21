use std::marker::PhantomData;

fn main() {}

trait Props<'parent> {}

struct SomeProps<'p> {
    text: &'p str,
}

impl<'p> Props<'p> for SomeProps<'p> {}

struct OutputNode<'a> {
    _p: PhantomData<&'a ()>,
}

// combine reference to self (borrowed from self) and referenfce to parent (borrowed from parent)
// borrow chain looks like 'p + 's -> 'p + 's -> 'p + 's
// always adding new lifetimes from self into the mix
// what does a "self" lifetime mean?
// a "god" gives us our data
// the god's lifetime is tied to Context, and the borrowed props object
// for the sake of simplicity, we just clobber lifetimes.
// user functions are just lies and we abuse lifetimes.
// everything is managed at runtime because that's how we make something ergonomc
// lifetime management in dioxus is just cheating around the rules
// our kind god manages lifetimes for us so we don't have to, thanks god
fn something<'s>(props: &'s SomeProps<'s>) -> OutputNode<'s> {
    todo!()
}

// type BC<'p, P: Props<'p>> = for<'a, 'b, 'c> fn(&'a P<'b>) -> OutputNode<'c>;
