use crate::{Read, Signal};
use dioxus_core::prelude::*;

#[doc(hidden)]
pub struct ReadFromMarker<M>(std::marker::PhantomData<M>);

impl<T, O, M> SuperFrom<T, ReadFromMarker<M>> for Read<O>
where
    O: SuperFrom<T, M> + 'static,
{
    fn super_from(input: T) -> Self {
        Read::new(Signal::new(O::super_from(input)))
    }
}

#[test]
#[allow(unused)]
fn into_signal_compiles() {
    fn takes_signal_string<M>(_: impl SuperInto<Read<String>, M>) {}

    fn takes_option_signal_string<M>(_: impl SuperInto<Read<Option<String>>, M>) {}

    fn don_t_run() {
        takes_signal_string("hello world");
        takes_signal_string(Signal::new(String::from("hello world")));
        takes_option_signal_string("hello world");
    }
}
