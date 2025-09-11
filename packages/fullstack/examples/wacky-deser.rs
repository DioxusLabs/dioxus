fn main() {
    let res = (&&&Targ::default()).do_thing(());
    let res = (&&&Targ::default()).do_thing(123);
}

struct Targ<T> {
    _marker: std::marker::PhantomData<T>,
}
impl<T> Targ<T> {
    fn default() -> Self {
        Targ {
            _marker: std::marker::PhantomData,
        }
    }
}

trait DoThing<M> {
    type Input;
    type Output;
    fn do_thing(&self, input: Self::Input) -> Self::Output;
}

impl DoThing<()> for &&Targ<()> {
    type Output = i32;
    type Input = ();
    fn do_thing(&self, _input: Self::Input) -> Self::Output {
        42
    }
}

impl DoThing<i32> for &Targ<i32> {
    type Output = String;
    type Input = i32;
    fn do_thing(&self, input: Self::Input) -> Self::Output {
        input.to_string()
    }
}
